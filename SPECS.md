# SPECS.md — Device Capture System

This document specifies the intended design of the system: the deployment topologies it must
support, the roles each component plays, the wire protocol, and the current implementation state
against that spec. It consolidates the architecture described in `README.md`/`AGENT.md` with the
findings in `CODE_REVIEW.md` (historical review at commit `40c8af5`; the status in §6 and §8 was revised
2026-10-30 against commit `3c3d183`, with `cargo test --workspace`, the Python pytest and a device-side hardware run).

Status markers used throughout: ✅ done · ⚠️ partial/broken · ❌ not implemented.

---

## 1. Purpose

Capture synchronized video (v4l2) and audio (ALSA) from one or more machines on a LAN, transport
raw frames with minimal added latency, aggregate and time-synchronize them centrally, and expose
the combined stream to consumers — including non-Rust consumers (Python) — for downstream use
(e.g. multi-camera calibration/recording pipelines).

---

## 2. Logical roles

The system is defined in terms of five logical roles, not five physical machines or five crates.
Any role may be colocated with any other role on the same host (talking over `127.0.0.1`) or
split across hosts (talking over the LAN) — the transport layer doesn't distinguish the two.

| Role | Responsibility | Carried by |
|---|---|---|
| **Producer** | Owns a hardware device (camera or mic), runs its capture loop, publishes `FramePacket`s | `device-node` bin + `devices` crate |
| **Receiver** | Terminates producer connections, subscribes to their published frames | `control-hub` (via `networking::Connection`/`Proxy`'s XSUB side) |
| **Accumulator** | Fans in frames from all connected producers into one logical stream | `control-hub` (via `networking::Proxy`'s XSUB side, one connection per `HUB_PRODUCERS` endpoint) |
| **Syncer** | Aligns frames from different producers/devices to a common clock/reference | `control-hub` (planned — see §6) |
| **Consumer** | Subscribes to the hub's aggregated output and does something with the frames | `access-client` bin, and/or third-party clients (e.g. Python) |

There is currently no separate "syncer" crate — sync is a planned responsibility *inside*
`control-hub`, not a standalone deployable unit. If a future requirement needs sync to scale or
fail independently of the hub's fan-in, it would become its own crate; nothing today requires
that split.

---

## 3. Deployment topologies

The topologies below must be reachable by changing only IP/port configuration — never by
recompiling or forking code paths. §3.1, §3.2 and §3.3 (several producer hosts, via `HUB_PRODUCERS`) all meet this today.
The reason this is
possible is that `Sender`, `Receiver`, and `Proxy`
(the three `networking` transport primitives) are parameterized purely by `Ipv4Addr` + `u16` and
use plain TCP, which works identically over loopback or a real NIC.

### 3.1 One device
Camera/mic producer, receiver, accumulator, syncer, and consumer all run as separate OS processes
on one machine, wired together over `127.0.0.1`:

```
[device-node] --tcp(lo)--> [control-hub: receiver+accumulator+syncer] --tcp(lo)--> [access-client]
```

Use case: development, single-camera local recording, demoing without a LAN.

### 3.2 Two devices
Producer(s) on host A; receiver + accumulator + syncer + consumer colocated on host B:

```
Host A: [device-node (camera/mic)] ---LAN--->  Host B: [control-hub] --tcp(lo)--> [access-client]
```

`access-client` on host B is just a second local process subscribing to `control-hub`'s XPUB over
loopback — it does not need to be merged into the hub binary.

### 3.3 Three devices
Producer, hub, and consumer each on their own machine on the same network — this is the
architecture's design center and matches the README diagram directly:

```
Host A: [device-node]  ---LAN--->  Host B: [control-hub]  ---LAN--->  Host C: [access-client]
```

Multiple producer hosts (A1, A2, …) are expected to fan in to the same hub — this is the reason
`control-hub` exists as a distinct XSUB/XPUB proxy rather than `access-client` connecting to each
`device-node` directly.

**Multi-producer fan-in is implemented:** the hub connects its XSUB socket to every endpoint listed in
`HUB_PRODUCERS` (`ip:port,ip:port`), falling back to `HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` when unset
(§8 item 13). Opt-in hub sync is partial (§8 item 10).

### 3.4 What's shared across all three
In every topology the wire protocol and the per-role code are identical; only bind/connect
addresses differ. No topology requires a code fork, a build flag, or a different binary.

---

## 4. Transport design: why ZeroMQ, not WebRTC

The system uses raw ZeroMQ PUB/SUB (producer→hub) and XSUB/XPUB (hub fan-in/fan-out) over TCP,
carrying bincode-encoded headers and unencoded frame bytes.

This is the right choice for the stated goal (lowest possible latency on a trusted LAN) and
should **not** be replaced with WebRTC:

- **No negotiation overhead.** WebRTC requires SDP offer/answer and ICE candidate gathering before
  a single frame flows; ZMQ just connects a TCP socket.
- **No NAT traversal machinery.** STUN/TURN exist to solve a problem (NAT/firewall traversal
  between untrusted peers) this system doesn't have — all hosts are on the same LAN and under
  common administration.
- **No mandatory encryption.** DTLS/SRTP are built into WebRTC and cost CPU and latency on every
  frame; a LAN capture rig can run unencrypted or add a cheaper transport-level option (e.g.
  CurveZMQ) only if actually required.
- **No codec requirement.** WebRTC's media pipeline assumes encoded video (VP8/H.264/etc.). This
  system intentionally sends raw v4l2 buffers and raw PCM — any codec step would add encode
  latency the design doesn't want and would complicate frame-accurate sync.
- **No jitter buffer / adaptive bitrate.** Those exist to smooth over consumer-internet conditions;
  on a LAN they add latency to solve a problem that doesn't exist here, and they interfere with
  the deterministic frame-interval assumption sync will rely on.

ZMQ's own trade-off — PUB silently drops frames once a subscriber's high-water mark is hit, with
no log or counter (§6, and CODE_REVIEW §3) — is the one real cost of this choice and must be
instrumented, not papered over by switching transports.

**Conclusion:** ZMQ is architecturally correct for this system's latency goal. Single-producer
forwarding over it works (§6). The open problems are sync, hardware validation of device capture,
and calibration, not the transport choice. Multi-producer fan-in and remote control are implemented.

---

## 5. Third-party integration (Python et al.)

`README.md`'s TODO already specifies the target API:

```python
ds = DeviceSystem(hub_ip, hub_port)
for frame in ds.get_frame():
    ...
```

This API exists today as PyO3 bindings in `python-client/`, exporting a `device_system` extension
module with `DeviceSystem` (hub connection + iterator) and `Frame` (header + raw bytes). It wraps
the Rust `access-client` so Python shares the exact wire decoder.

Packaging is in place: `python-client/pyproject.toml` builds the extension with maturin, and
`python-client/tests/test_loopback.py` checks it end to end (item 15).

The implemented path is PyO3 bindings in `python-client/` (see `python-client/README.md`):
`DeviceSystem` wraps `access-client::AccessClient`, so Python uses the same Rust wire decoder and
cannot drift from it when `shared` changes. A pure-Python decoder (a `pyzmq` `SUB` socket plus a small
bincode header decoder, not written and not specified here) remains a fallback only if consumers must avoid a Rust-built
extension. It is not planned.

---

## 6. Current implementation state

Revised 2026-10-29 against the current workspace (`e367b77`). `CODE_REVIEW.md` is a historical review at commit
`40c8af5`; see it for line numbers and the findings it already applied (for example the `devices`
crate's broken module re-export, §4.1 there).

| Component | State | Blocks which role(s) |
|---|---|---|
| `shared` (data model) | ✅ Works: `ConnectionStats` uses `Instant` for interval math and records per-frame latency | Observability for all roles |
| `networking::Connection`/`Sender`/`Receiver` | ✅ Works: receive path validates peer input and drops malformed frames; a sequence part enables gap-based dropped-frame counts | Producer, Consumer |
| `networking::Proxy` | ✅ Works: `start()` runs `zmq::proxy_steerable` on a forwarding thread; `stop()` terminates it cleanly through a control socket. Covered by `proxy-test.rs` and the hub pipeline test | Receiver + Accumulator |
| `devices::V4lCameraDevice` | ✅ Streams on a real webcam at the granted rate in every advertised mode (2026-10-30 survey, `docs/hardware/`): `start()` disables exposure auto-priority, sets format before interval and reads both back; `device-node` selects the mode from `get_configs()` (§7.3). End-to-end 60 s run still open (item 12) | Producer (camera) |
| `devices::AlsaMicrophoneDevice` | ✅ Enumerates one converting PCM per card plus sound-server PCMs (`select_capture_hints`, item 20) and captures 16 kHz mono from both on real hardware (2026-10-30, `docs/hardware/`). Sample rate is not yet on the wire (item 21) | Producer (mic) |
| `devices::microphone_windows` | ❌ Empty file | Producer on Windows |
| `devices::manager` | ✅ Deleted | n/a |
| `device-node::DeviceNode` | ✅ Works: generic over `T: Device`, can publish one camera/mic stream, serves `GET /status` and remote `POST /start`, `/stop`, `/restart` with transition checks | Producer |
| `control-hub` | ✅ Works as XSUB/XPUB proxy fanning in every `HUB_PRODUCERS` endpoint (or the single `HUB_PRODUCER_*` pair), configured by `HUB_*` env vars; serves `GET /status` with live producer and consumer counts; opt-in `HUB_SYNC_PORT` runs an `AlignedRelay` that publishes aligned groups on a separate PUB port while the raw forwarding stays unchanged | Syncer |
| `shared::Aligner` | ✅ Generic over a payload; groups frames from several devices by capture time, reports skew, and counts dropped stale samples. Used by the hub's `AlignedRelay` and by `AlignedClient` | Syncer |
| `access-client` | ✅ `AccessClient` (`connect`, `next_frame`, `run`, `disconnect`, `dropped_frames`) plus a CLI binary with a per-device summary mode (`CLIENT_SUMMARY_SECS`, `CLIENT_RUN_SECS`: rate, bandwidth, latency, drops); `AlignedClient` wraps it to yield aligned groups of frame packets, covered by end-to-end tests. Used by Python bindings | Consumer |
| `python-client` | ✅ PyO3 bindings (`device_system`) packaged with maturin and covered by a pytest that runs the loopback example | Consumer |
| `DeviceStatus` | ✅ Status transitions are implemented and observable via `device-node` | Observability for Producer |

### 6.1 What already satisfies the topology requirement (§3)
The *shape* of the deployment flexibility is real: `Sender::new`/`Receiver::new`/`Proxy::new` take
`Ipv4Addr` + `u16` with no hardcoded host assumptions, and `device-node`/`control-hub`/`access-client`
are configured through arguments and environment variables only. Topologies §3.1, §3.2 and §3.3 (with
several producers via `HUB_PRODUCERS`) are reachable through configuration alone. Sync (§2 Syncer) is partial: an opt-in `HUB_SYNC_PORT` relay exists and is covered only by tests.

### 6.2 What blocks it today
Resolved: `networking::Proxy` forwards data (`proxy_steerable`) and `control-hub` runs it;
`DeviceNode` is generic over `T: Device`, status transitions are implemented, and `/status` is served.
`cargo test --workspace` and the Python loopback pytest pass at `3c3d183` (2026-10-30), and again after items 19,
20 and 12 landed the same day.

Resolved since the last review: multi-producer fan-in (§8 item 13), covered by the two-producer proxy
and hub pipeline tests.

Open, in priority order:
0. ✅ Producers on real hardware: item 19 (camera selection, FourCC on the wire, frame-rate fixes) and item 20
   (microphone enumeration on PipeWire hosts) are done; item 21 (sample rate on the wire) is a small follow-up.
1. ⚠️ Sync and calibration: the hub runs an opt-in `AlignedRelay` (`HUB_SYNC_PORT`), and consumers can use `AlignedClient`; the relay has not been validated on real devices, and calibration is not started (§2 Syncer).
2. ✅ Hardware validation (item 12): 70 s single-host run through the full path, camera 30.0 fps and microphone
   16 kHz with 0 dropped frames (`docs/hardware/`). A cross-host run is still outstanding.
3. ✅ Remote control: `POST /start`, `/stop` and `/restart` are implemented on the device-node status server (§7.1, item 14).
4. ✅ Python packaging and tests: maturin `pyproject.toml` and a loopback pytest (§5, item 15).
5. ❌ Windows microphone: `devices/src/microphone_windows.rs` is empty.

---

## 7. Wire protocol (frozen contract — anything built on top, including §5's Python client,
depends on this)

A frame is one ZMQ multipart message:

| Part | Content |
|---|---|
| `0` | `bincode::encode_to_vec(FramePacketInformation, config::standard())` |
| `1` | Sender-local `u64` sequence number (little-endian) |
| `2..n` | Raw frame bytes, split into `data_chunk_size`-byte chunks (receiver concatenates them back; chunk boundaries carry no meaning) |

`FramePacketInformation`:

```rust
pub struct FramePacketInformation {
    pub device_info: DeviceInformation,   // id, name, device_type, device_status
    pub rx_timestamp: Option<SystemTime>, // set by receiver on arrival
    pub tx_timestamp: Option<SystemTime>, // set by sender just before send
    pub frame_shape: Vec<u32>,
    pub pixel_format: Option<String>,     // camera: negotiated FourCC ("YUYV", "MJPG", ...); microphone: None
}
```

`pixel_format` was added 2026-10-30 (item 19). Every peer in a deployment must be built from the same revision;
there is no protocol version negotiation, and a pre-item-19 peer cannot decode frames from a post-item-19 peer.

Payload formats by device type:

- **Camera** — the v4l2 buffer exactly as the camera delivered it, in the FourCC named by `pixel_format`,
  `frame_shape = [width, height]`. The format is whichever advertised mode the device node selected (§7.3), so
  consumers must branch on `pixel_format`: `YUYV` is 2 bytes per pixel, `MJPG` is one JPEG image per frame.
- **Microphone** — interleaved `S16LE` samples as bytes, `frame_shape = [buffer_size, channels]`
  (channel count is included). **Gap (item 21):** the sample rate is not on the wire; consumers assume the node's
  configured rate (default 16 kHz).

Per-frame latency is measurable today as `rx_timestamp - tx_timestamp`. The receive path records
this into `ConnectionStats` (best-effort; frames without a `tx_timestamp` are dropped).

Defaults (`device-node/src/helper.rs`):

| Device | Default config |
|---|---|
| Camera | Selected from the camera's advertised modes (§7.3); the default request is 1920×1080 at 30 fps, uncompressed preferred on ties |
| Microphone | 16kHz, 1 channel, 512-sample buffer, `S16LE` |

### 7.1 Device-node status endpoint

`GET /status` on `DEVICE_NODE_STATUS_ADDR` (default `127.0.0.1:8080`) returns `200` with
`Content-Type: application/json`:

```json
{"device":"Running","connection":"Active"}
```

The values are the `Debug` names of `DeviceStatus` and `ConnectionStatus`. Other paths and methods return `404`.
A failed state read returns `500` with `{"error":"status unavailable"}`.

Remote control on the same server (§8 item 14):

- `POST /start`, `POST /stop`, `POST /restart` transition the device and respond with the new status in the
  same JSON format.
- A transition the current `DeviceStatus` does not allow (`/start` while `Running`, `/stop` or `/restart`
  while not `Running`, or any control on a node that was never started) returns `409` with
  `{"error":"cannot <Action> while device is <Status>"}`.
- A device failure during control returns `500` with `{"error":"device control failed"}`.

The server validates transitions before calling the device, so a bad request cannot trigger the device's
own start/stop assertions.

### 7.3 Camera config selection

`device-node run camera` never streams a fixed config. It enumerates the camera's advertised modes
(`Device::get_configs`) and picks one with `device_node::CameraSelection`:

| `DEVICE_NODE_CAMERA_POLICY` | Rule | Extra variables |
|---|---|---|
| `nearest` (default) | `shared::select_camera_config`: closest resolution, then closest frame rate; an uncompressed FourCC wins an exact tie | `DEVICE_NODE_CAMERA_WIDTH`, `_HEIGHT`, `_FPS` (defaults 1920, 1080, 30) |
| `best-fps` | `shared::select_camera_config_by_policy(BestFps)` | none |
| `best-resolution` | `…(BestResolution)` | none |
| `best-average` | `…(BestAverage)` | none |

The node prints the selection and the chosen config at start. `V4lCameraDevice::start` still refuses to run if
the driver negotiates anything other than the chosen config, so a mismatch is an error, never a silent change.

### 7.2 Environment configuration

The binaries read these variables (see `README.md`, Configuration):

| Variable | Binary | Default |
|---|---|---|
| `DEVICE_NODE_STATUS_ADDR` | `device-node` | `127.0.0.1:8080` |
| `DEVICE_NODE_CAMERA_POLICY` | `device-node` | `nearest` (§7.3) |
| `DEVICE_NODE_CAMERA_WIDTH` / `_HEIGHT` / `_FPS` | `device-node` | `1920` / `1080` / `30`, used by `nearest` |
| `HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` | `control-hub` | `127.0.0.1` / `5555` |
| `HUB_CONSUMER_IP` / `HUB_CONSUMER_PORT` | `control-hub` | `0.0.0.0` / `5556` |
| `HUB_QUEUE_SIZE` | `control-hub` | `128` |
| `HUB_DATA_CHUNK_SIZE` | `control-hub` | `1024` |
| `HUB_PRODUCERS` | `control-hub` | unset; comma-separated `ip:port` list that takes precedence over `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT` |
| `HUB_STATUS_ADDR` | `control-hub` | `127.0.0.1:8081` |
| `HUB_SYNC_PORT` | `control-hub` | unset (sync off); when set, `HUB_SYNC_DEVICES` is required |
| `HUB_SYNC_DEVICES` | `control-hub` | unset; device ids to align, required with `HUB_SYNC_PORT` |

---

## 8. Build order and remaining work

Items 1–9, 11–16, 19 and 20 are complete. Item 10 is partial; items 17, 18 and 21 are open.
Items 19–21 were added 2026-10-30 from the first hardware run. Item numbers
are stable so older references still resolve. Each item's spec, acceptance criteria and checklist is in its
own file under [`docs/specs/`](docs/specs/README.md), grouped into stages. §8.1 gives the order in which to
implement the open items.

| Item | Stage | Status | Spec file |
|---|---|---|---|
| 1 | 1 — Transport and hub foundation | ✅ | [step-01](docs/specs/01-transport-and-hub/step-01-fix-proxy.md) |
| 2 | 1 — Transport and hub foundation | ✅ | [step-02](docs/specs/01-transport-and-hub/step-02-control-hub-main.md) |
| 3 | 4 — Consumers and Python | ✅ | [step-03](docs/specs/04-consumers-and-python/step-03-access-client-frames.md) |
| 4 | 2 — Producers | ✅ | [step-04](docs/specs/02-producers/step-04-device-enumeration-filters.md) |
| 5 | 1 — Transport and hub foundation | ✅ | [step-05](docs/specs/01-transport-and-hub/step-05-receive-path-panics.md) |
| 6 | 2 — Producers | ✅ | [step-06](docs/specs/02-producers/step-06-device-status-endpoint.md) |
| 7 | 2 — Producers | ✅ | [step-07](docs/specs/02-producers/step-07-generic-device-node.md) |
| 8 | 2 — Producers | ✅ | [step-08](docs/specs/02-producers/step-08-camera-buffers-format.md) |
| 9 | 1 — Transport and hub foundation | ✅ | [step-09](docs/specs/01-transport-and-hub/step-09-connection-stats.md) |
| 10 | 5 — Sync | ⚠️ | [step-10](docs/specs/05-sync/step-10-time-alignment.md) |
| 11 | 4 — Consumers and Python | ✅ | [step-11](docs/specs/04-consumers-and-python/step-11-pyo3-bindings.md) |
| 12 | 6 — Hardware validation | ✅ | [step-12](docs/specs/06-hardware-validation/step-12-hardware-validation.md) |
| 13 | 3 — Hub fan-in and hub status | ✅ | [step-13](docs/specs/03-hub-fan-in-and-status/step-13-multi-producer-fan-in.md) |
| 14 | 2 — Producers | ✅ | [step-14](docs/specs/02-producers/step-14-remote-control.md) |
| 15 | 4 — Consumers and Python | ✅ | [step-15](docs/specs/04-consumers-and-python/step-15-python-packaging-tests.md) |
| 16 | 3 — Hub fan-in and hub status | ✅ | [step-16](docs/specs/03-hub-fan-in-and-status/step-16-hub-status-endpoint.md) |
| 17 | 7 — Calibration | ❌ | [step-17](docs/specs/07-calibration/step-17-calibration.md) |
| 18 | 8 — Windows microphone | ❌ | [step-18](docs/specs/08-windows-microphone/step-18-windows-microphone.md) |
| 19 | 2 — Producers | ✅ | [step-19](docs/specs/02-producers/step-19-camera-config-negotiation.md) |
| 20 | 2 — Producers | ✅ | [step-20](docs/specs/02-producers/step-20-microphone-enumeration-pipewire.md) |
| 21 | 2 — Producers | ❌ | [step-21](docs/specs/02-producers/step-21-microphone-sample-rate-on-the-wire.md) |

### 8.1 Order of remaining work

Implement the open items in this order: 21 → 10 → 17 → 18. Item 21 is a small wire-format follow-up and
should land before any external consumer depends on the header. An item is done only
when its acceptance check passes with `cargo test --workspace` (plus the hardware run for item 12), and
the README TODO and this file are updated in the same change.

---

## 9. Non-goals (explicitly out of scope per the design above)

- WebRTC, or any browser-interop transport — see §4.
- NAT traversal / cross-internet deployment — the system assumes a trusted LAN.
- Per-frame encryption by default — not required on a trusted LAN; revisit only if a specific
  threat model demands it, and prefer a transport-level option (CurveZMQ) over switching protocols.
- Video/audio codecs in the hot path — this system never encodes: frames are transported exactly as the device
  delivered them, to avoid added encode latency and to keep frame-accurate sync tractable. A camera that only
  offers a mode as MJPG (every UVC webcam does at 1080p, see `docs/hardware/`) is still streamed in that mode, with
  `pixel_format = "MJPG"` on the wire; decoding is the consumer's choice. Consumers that need uncompressed frames
  ask for a mode the camera offers uncompressed (`DEVICE_NODE_CAMERA_WIDTH=640 …`), they do not get a transcode
  from the node. Decided 2026-10-30 (item 19).
