# SPECS.md — Device Capture System

This document specifies the intended design of the system: the deployment topologies it must
support, the roles each component plays, the wire protocol, and the current implementation state
against that spec. It consolidates the architecture described in `README.md`/`AGENT.md` with the
findings in `CODE_REVIEW.md` (historical review at commit `40c8af5`; the status in §6 was revised
2026-10-28 against commit `7ac4b5a`).

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
recompiling or forking code paths. §3.1 and §3.2 (one producer) meet this today; §3.3 with more than
one producer host also needs multi-producer fan-in, which is not implemented yet. The reason this is
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
(§8 item 13). Sync across producers is still missing (§8 item 10).

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
bincode header decoder, specified in §7) remains a fallback only if consumers must avoid a Rust-built
extension. It is not planned.

---

## 6. Current implementation state

Revised against the current workspace (`7ac4b5a`). `CODE_REVIEW.md` is a historical review at commit
`40c8af5`; see it for line numbers and the findings it already applied (for example the `devices`
crate's broken module re-export, §4.1 there).

| Component | State | Blocks which role(s) |
|---|---|---|
| `shared` (data model) | ✅ Works: `ConnectionStats` uses `Instant` for interval math and records per-frame latency | Observability for all roles |
| `networking::Connection`/`Sender`/`Receiver` | ✅ Works: receive path validates peer input and drops malformed frames; a sequence part enables gap-based dropped-frame counts | Producer, Consumer |
| `networking::Proxy` | ✅ Works: `start()` runs `zmq::proxy_steerable` on a forwarding thread; `stop()` terminates it cleanly through a control socket. Covered by `proxy-test.rs` and the hub pipeline test | Receiver + Accumulator |
| `devices::V4lCameraDevice` | ⚠️ Compiles, enumerates: capability filtering is in place, buffer count reduced, negotiated-format validation added; capture path still needs real-hardware exercise | Producer (camera) |
| `devices::AlsaMicrophoneDevice` | ⚠️ Compiles, enumerates: enumeration filters fixed and buffer sizing accounts for channels; capture path still needs real-hardware exercise | Producer (mic) |
| `devices::microphone_windows` | ❌ Empty file | Producer on Windows |
| `devices::manager` | ✅ Deleted | n/a |
| `device-node::DeviceNode` | ✅ Works: generic over `T: Device`, can publish one camera/mic stream, serves `GET /status` and remote `POST /start`, `/stop`, `/restart` with transition checks | Producer |
| `control-hub` | ⚠️ Works as XSUB/XPUB proxy fanning in every `HUB_PRODUCERS` endpoint (or the single `HUB_PRODUCER_*` pair), configured by `HUB_*` env vars; sync and calibration are not implemented | Syncer (missing) |
| `access-client` | ✅ `AccessClient` (`connect`, `next_frame`, `run`, `disconnect`) plus a CLI binary; used by Python bindings | Consumer |
| `python-client` | ✅ PyO3 bindings (`device_system`) packaged with maturin and covered by a pytest that runs the loopback example | Consumer |
| `DeviceStatus` | ✅ Status transitions are implemented and observable via `device-node` | Observability for Producer |

### 6.1 What already satisfies the topology requirement (§3)
The *shape* of the deployment flexibility is real: `Sender::new`/`Receiver::new`/`Proxy::new` take
`Ipv4Addr` + `u16` with no hardcoded host assumptions, and `device-node`/`control-hub`/`access-client`
are configured through arguments and environment variables only. Topologies §3.1, §3.2 and §3.3 (with
several producers via `HUB_PRODUCERS`) are reachable through configuration alone. Sync (§2 Syncer) is
still missing.

### 6.2 What blocks it today
Resolved: `networking::Proxy` forwards data (`proxy_steerable`) and `control-hub` runs it;
`DeviceNode` is generic over `T: Device`, status transitions are implemented, and `/status` is served.

Resolved since the last review: multi-producer fan-in (§8 item 13), covered by the two-producer proxy
and hub pipeline tests.

Open, in priority order:
1. ❌ Sync and calibration: no code exists (§2 Syncer).
2. ⚠️ Hardware validation: the v4l2 and ALSA capture loops compile and have hardware-free tests, but
   have not been run end to end against real devices.
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
}
```

Payload formats by device type:

- **Camera** — raw v4l2 buffer in the negotiated FourCC (default `YUYV`), `frame_shape = [width, height]`.
- **Microphone** — interleaved `S16LE` samples as bytes, `frame_shape = [buffer_size, channels]`
  (channel count is included).

Per-frame latency is measurable today as `rx_timestamp - tx_timestamp`. The receive path records
this into `ConnectionStats` (best-effort; frames without a `tx_timestamp` are dropped).

Defaults (`device-node/src/helper.rs`):

| Device | Default config |
|---|---|
| Camera | 1920×1080, 30fps, `YUYV` |
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

### 7.2 Environment configuration

The binaries read these variables (see `README.md`, Configuration):

| Variable | Binary | Default |
|---|---|---|
| `DEVICE_NODE_STATUS_ADDR` | `device-node` | `127.0.0.1:8080` |
| `HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` | `control-hub` | `127.0.0.1` / `5555` |
| `HUB_CONSUMER_IP` / `HUB_CONSUMER_PORT` | `control-hub` | `0.0.0.0` / `5556` |
| `HUB_QUEUE_SIZE` | `control-hub` | `128` |
| `HUB_DATA_CHUNK_SIZE` | `control-hub` | `1024` |

---

## 8. Build order and remaining work

Items marked ✅ are done. Items 1–9, 11, and 13–15 are complete. Items 10, 12 and 16–18 are open. Item numbers
are stable so older references still resolve. §8.1 gives the order in which to implement the open
items.

1. ✅ Done: fix `Proxy` and add loopback proxy tests (`proxy-test.rs`, hub pipeline test).
2. ✅ Done: minimal `control-hub` `main()` loads config and runs `Proxy`.
3. ✅ Done: `access-client` connects a `Receiver` to the hub and exposes frames via
   `AccessClient`. Still open: packaging the Python bindings that sit on top of it (§5).
4. ✅ Done: microphone enumeration filter fixed (playback-only skipped, virtual devices matched by
   base name) and camera capability filtering (`V4L2_CAP_VIDEO_CAPTURE`) added. Build-verified
   with a Nix shell providing libclang; the hardware itself has not been exercised.
5. ✅ Done: remove panics on the network receive path (peer-controlled input is validated and
   malformed frames are dropped).
6. ✅ Done: camera and microphone set `DeviceStatus` to `Running` on start and `Stopped` on stop,
   and `device-node run` serves `GET /status` (JSON with device and connection state) on
   `DEVICE_NODE_STATUS_ADDR` (default `127.0.0.1:8080`). Covered by `device-node/tests/status-test.rs`.
7. ✅ Done: `DeviceNode` is generic over `T: Device`, so producer hosts can publish cameras.
8. ✅ Done: camera buffer count reduced from 128 to 4, and the negotiated v4l2 format is validated.
9. ✅ Done: rewrite `ConnectionStats` to use `Instant` for interval math and record per-frame latency
   from `tx_timestamp`/`rx_timestamp`.
10. ⚠️ Sync: time alignment across producers. The grouping core `shared::Aligner` exists and is covered
    by unit tests with synthetic timestamps (groups within a tolerance, reports skew, drops stale
    samples). It is not yet wired into `control-hub`, so the hub still forwards frames unaligned.
    Calibration is item 17.
11. ✅ Done: Python client (§5) as a PyO3 extension in `python-client/`. `DeviceSystem(hub_ip, hub_port)`
    with `get_frame()` / `next_frame()` wraps `AccessClient`, so Python shares the Rust wire decoder.
    Packaging and tests are covered by item 15.
12. ❌ Hardware validation: run the `V4lCameraDevice` and `AlsaMicrophoneDevice` capture loops against
    real devices, end to end through `device-node` → `control-hub` → `access-client`. Acceptance: frames
    arrive at the configured rate (30 fps camera, 16 kHz microphone) for at least 60 s, and the dropped
    frame count and latency are recorded in a dated results note. Tests stay hardware-free.
13. ✅ Done: multi-producer fan-in. `control-hub` connects its XSUB socket to several producer endpoints,
    for example `HUB_PRODUCERS=ip:port,ip:port`, keeping the single-endpoint variables as a fallback.
    Acceptance: a hardware-free test runs two producers on different loopback ports and checks that
    `access-client` receives frames with both device IDs.
14. ✅ Done: remote control. `device-node` accepts `POST /start`, `POST /stop` and `POST /restart` on the
    status server, with transitions validated against `DeviceStatus` (§7.1). Covered by
    `device-node/tests/status-test.rs` for each transition and each invalid transition (409).
15. ✅ Done: Python packaging and tests. `python-client/pyproject.toml` builds the extension with maturin, and
    `python-client/tests/test_loopback.py` runs `examples/loopback-publisher.rs` and receives frames through
    `DeviceSystem`. Verified: a clean virtualenv installed the package and the pytest passed.
16. ❌ Hub status endpoint: `control-hub` serves `GET /status` with producer and consumer counts. This is
    the planned hub HTTP API from the README diagram. Acceptance: a test queries it while one producer
    and one consumer are connected.
17. ❌ Calibration: choose a general reference-point detection model, then port the python-opencv
    calibration code. Acceptance: a deterministic test on a checked-in fixture image returns the expected
    reference points.
18. ❌ Windows microphone: implement `devices/src/microphone_windows.rs`, or record in the README
    Requirements that Windows is unsupported. Acceptance: one of the two is written down, and if
    implemented, the device tests pass on Windows. Lowest priority.

### 8.1 Order of remaining work

Implement the open items in this order: 12 → 10 → 15 → 16 → 17 → 18. An item is done only
when its acceptance check passes with `cargo test --workspace` (plus the hardware run for item 12), and
the README TODO and this file are updated in the same change.

---

## 9. Non-goals (explicitly out of scope per the design above)

- WebRTC, or any browser-interop transport — see §4.
- NAT traversal / cross-internet deployment — the system assumes a trusted LAN.
- Per-frame encryption by default — not required on a trusted LAN; revisit only if a specific
  threat model demands it, and prefer a transport-level option (CurveZMQ) over switching protocols.
- Video/audio codecs in the hot path — frames are transported raw to avoid encode/decode latency
  and to keep frame-accurate sync tractable.
