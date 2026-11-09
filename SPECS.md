# SPECS.md — Device Capture System

This document specifies the intended design of the system: the deployment topologies it must
support, the roles each component plays, the wire protocol, and the current implementation state
against that spec. It consolidates the architecture described in `README.md`/`AGENT.md` with the
findings in `docs/history/CODE_REVIEW.md` (historical review at commit `40c8af5`; the status in §6 and §8 was revised
2026-10-30 against commit `3c3d183`, with `cargo test --workspace`, the Python pytest and a device-side hardware run).

Status markers used throughout: ✅ done · ⚠️ partial/broken · ❌ not implemented.

---

## 1. Purpose

Capture synchronized video (v4l2) and audio (ALSA) from one or more machines on a LAN, transport
raw frames with minimal added latency, aggregate and time-synchronize them centrally, and expose
the combined stream to consumers — including non-Rust consumers (Python) — for downstream use.

The project's concern ends at delivering images. It extracts the necessary images, prepares them with
configurable basic transforms (item 23), and hands them over in one of two ways: recorded to video files with
per-frame timestamps (item 24), or live to another application (item 25). Calibration (items 17, 22) is
included because it describes the cameras that produced the images.

The target downstream application is a human pose pipeline: instance segmentation of people, per-person
body-part and SMPL-X extraction, body-part classification, and refinement with keypoints. That pipeline is a
separate project and a consumer of this one; none of its stages are built here (§9).

---

## 2. Logical roles

The system is defined in terms of six logical roles, not six physical machines or six crates.
Any role may be colocated with any other role on the same host (talking over `127.0.0.1`) or
split across hosts (talking over the LAN) — the transport layer doesn't distinguish the two.

| Role | Responsibility | Carried by |
|---|---|---|
| **Producer** | Owns a hardware device (camera or mic), runs its capture loop, publishes `FramePacket`s | `device-node` bin + `devices` crate |
| **Receiver** | Terminates producer connections, subscribes to their published frames | `control-hub` (via `networking::Connection`/`Proxy`'s XSUB side) |
| **Accumulator** | Fans in frames from all connected producers into one logical stream | `control-hub` (via `networking::Proxy`'s XSUB side, one connection per `HUB_PRODUCERS` endpoint) |
| **Syncer** | Aligns frames from different producers/devices by capture time | `control-hub` (opt-in `AlignedRelay`, item 10), or a consumer with `AlignedClient` |
| **Consumer** | Subscribes to the hub's aggregated output and does something with the frames | `access-client` bin, `calibration`, the `recorder` and `live-sink` bins (items 24, 25, scaffolded), and third-party clients (e.g. Python) |
| **Pre-processor** | Applies configured basic image transforms per camera. It lives with the hub, never the producer: producers publish native frames once, and the hub's relay reads its own fanned-in stream | `preprocess` crate, run in `control-hub` (`networking::FrameRelay`) or in a consumer (item 23) |

There is no separate "syncer" crate — sync is a responsibility *inside* `control-hub` (an opt-in relay), not a
standalone deployable unit. If a future requirement needs sync to scale or
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
(§8 item 13). Opt-in hub sync is validated on real devices (§8 item 10).

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

ZMQ's own trade-off — PUB silently drops frames once a subscriber's high-water mark is hit — is the one real cost
of this choice and must be instrumented, not papered over by switching transports. It is instrumented: every frame
carries a sender sequence number, and receivers count the gaps (`dropped_frames`, the `access-client` summary,
the recorder's `session.json`).

**Conclusion:** ZMQ is architecturally correct for this system's latency goal. Forwarding, multi-producer
fan-in, remote control, hub-side sync and hardware validation are implemented and measured (§6, `docs/hardware/`).
Calibration runs as a consumer crate (`calibration/`), so the transport choice is not what limits it.

---

## 5. Third-party integration (Python et al.)

The target API from the README TODO exists as PyO3 bindings in [`python-client/`](python-client/README.md): a
`device_system` extension module with `DeviceSystem` (hub connection + iterator) and `Frame` (header + raw
bytes), built with maturin and covered by an end-to-end loopback pytest (item 15).

```python
ds = DeviceSystem(hub_ip, hub_port)
for frame in ds.get_frame():
    ...
```

`DeviceSystem` wraps `access-client::AccessClient`, so Python uses the exact Rust wire decoder and cannot
drift from it when `shared` changes. A pure-Python decoder (`pyzmq` plus a bincode header decoder) would be
the fallback if consumers must avoid a Rust-built extension. It is not planned.

## 6. Current implementation state

Revised 2026-11-09. `docs/history/CODE_REVIEW.md` is a historical review at commit
`40c8af5`; see it for line numbers and the findings it already applied (for example the `devices`
crate's broken module re-export, §4.1 there).

| Component | State | Blocks which role(s) |
|---|---|---|
| `shared` (data model) | ✅ Works: `ConnectionStats` uses `Instant` for interval math and records per-frame latency | Observability for all roles |
| `networking::Connection`/`Sender`/`Receiver` | ✅ Works: receive path validates peer input and drops malformed frames; a sequence part enables gap-based dropped-frame counts | Producer, Consumer |
| `networking::Proxy` | ✅ Works: `start()` runs `zmq::proxy_steerable` on a forwarding thread; `stop()` terminates it cleanly through a control socket. Covered by `proxy-test.rs` and the hub pipeline test | Receiver + Accumulator |
| `devices::V4lCameraDevice` | ✅ Streams on a real webcam at the granted rate in every advertised mode (2026-10-30 survey, `docs/hardware/`): `start()` disables exposure auto-priority, sets format before interval and reads both back; `device-node` selects the mode from `get_configs()` ([`device-node/README.md`](device-node/README.md)). Publishes the driver's `bytesused`, not the whole mmap buffer (fixed 2026-11-09: 1080p MJPG went from ~995 to ~105 Mbit/s) | Producer (camera) |
| `devices::AlsaMicrophoneDevice` | ✅ Enumerates one converting PCM per card plus sound-server PCMs (`select_capture_hints`, item 20), captures 16 kHz mono from both on real hardware (2026-10-30, `docs/hardware/`), and stamps the granted sample rate on every frame (item 21) | Producer (mic) |
| `devices::microphone_windows` | ✅ Removed; Windows is unsupported by decision (item 18) | n/a |
| `devices::manager` | ✅ Deleted | n/a |
| `device-node::DeviceNode` | ✅ Works: generic over `T: Device`, can publish one camera/mic stream, serves `GET /status` and remote `POST /start`, `/stop`, `/restart` with transition checks. `config save` / `run --config` start a host the same way every time, by device id and resolved mode (item 27) | Producer |
| `control-hub` | ✅ Works as XSUB/XPUB proxy fanning in every `HUB_PRODUCERS` endpoint (or the single `HUB_PRODUCER_*` pair), configured by `HUB_*` env vars; serves `GET /status` with live producer and consumer counts plus forwarded frames/s, Mbit/s and hub CPU (item 26 tooling); opt-in `HUB_SYNC_PORT` runs an `AlignedRelay` that publishes aligned groups on a separate PUB port while the raw forwarding stays unchanged. Validated on real devices 2026-10-30: one group per camera frame at 30/s, skew ≤ 18 ms | Syncer |
| `shared::Aligner` | ✅ Generic over a payload; groups frames from several devices by capture time, reports skew, and counts dropped samples. Pairing tolerance and wait window (`max_wait`) are separate; an anchor is dropped as soon as every other device has moved past it. Used by the hub's `AlignedRelay` and by `AlignedClient` | Syncer |
| `access-client` | ✅ `AccessClient` (`connect`, `next_frame`, `run`, `disconnect`, `dropped_frames`) plus a CLI binary with a per-device summary mode (`CLIENT_SUMMARY_SECS`, `CLIENT_RUN_SECS`: rate, bandwidth, latency, drops); `AlignedClient` wraps it to yield aligned groups of frame packets, covered by end-to-end tests. Used by Python bindings | Consumer |
| `python-client` | ✅ PyO3 bindings (`device_system`) packaged with maturin and covered by a pytest that runs the loopback example: `DeviceSystem` (raw and processed ports), `AlignedSystem` (sync port) and `Frame.image()` pixels through the `preprocess` decoder (item 28) | Consumer |
| `calibration` | ✅ Native Rust consumer crate (`opencv`): ChArUco reference-point detection, camera intrinsics, frame decoding (MJPG/YUYV); deterministic fixture test. Multi-camera extrinsics (item 22): board pose per camera, pose chaining to a reference camera, joint LM refinement, JSON calibration file; synthetic three-camera test recovers known poses to ~0.1 mm. ⚠️ Not yet run on real cameras | Calibration |
| `preprocess` | ✅ Per-camera transforms (rotate, flip, crop, resize, brightness, contrast, gamma, grayscale) from TOML; hub output on `HUB_PREPROCESS_PORT` via `FrameRelay`, or in `access-client`. Webcam: 640×480 at 30 fps / 13 ms, 1080p at 25.5 fps / 80 ms | Pre-processor |
| `recorder` | ✅ Records the raw, processed or aligned hub output: one `ffmpeg`-written video per camera (MJPG stream-copied, uncompressed input encoded), one WAV per microphone, a per-frame sidecar with wire timestamps and sequence numbers, `session.json` with drop and gap counts. Loopback integration tests; 30 s webcam recording with 0 drops (item 24, `docs/hardware/`) | Consumer |
| `live-sink` | ⚠️ Implemented, hardware run open: RGB3/GREY→YUYV conversion, V4L2 output writer, newest-frame forwarding loop, all tested against a loopback hub. The `v4l2loopback` round trip is an `#[ignore]` test; it and the webcam run need a host whose kernel ships the module (item 25) | Consumer |
| `DeviceStatus` | ✅ Status transitions are implemented and observable via `device-node` | Observability for Producer |

### 6.1 What already satisfies the topology requirement (§3)
The *shape* of the deployment flexibility is real: `Sender::new`/`Receiver::new`/`Proxy::new` take
`Ipv4Addr` + `u16` with no hardcoded host assumptions, and `device-node`/`control-hub`/`access-client`
are configured through arguments and environment variables only. Topologies §3.1, §3.2 and §3.3 (with
several producers via `HUB_PRODUCERS`) are reachable through configuration alone. Sync (§2 Syncer) is an opt-in
`HUB_SYNC_PORT` relay, covered by tests and validated on real devices (item 10).

### 6.2 What is open

Revised 2026-11-09. `cargo test --workspace` and the Python loopback pytest pass.

1. **Live sink on hardware** (item 25): the code is done and tested against a loopback hub; the `v4l2loopback`
   round trip and the webcam run need a host whose kernel ships the module.
2. **Multi-camera on hardware:** the extrinsics solver (item 22) is done and meets its synthetic acceptance
   criterion but has not seen real cameras; the multi-camera, cross-host run (item 26) is open. Its tooling is done
   (hub throughput and CPU on `/status`, `DEVICE_NODE_DATA_CHUNK_SIZE`, `scripts/multi-camera-run.sh` writing the
   results note). Both need a second camera; item 26 also needs a second host.

Known limits of finished work, recorded in the owning crates: pre-processing a 1080p MJPG stream reaches 22–25 fps,
not 30 (`preprocess/README.md`); the 1 KiB default chunk size has not been measured on the raw path at 1080p
(item 26); the hardware runs so far are single-host. The 2026-10-30 bandwidth and latency figures for 1080p MJPG
include the padding bug fixed 2026-11-09 (`docs/hardware/2026-11-09-recorder-saved-config.md`).

---

## 7. Per-crate specs

The detailed specification of each concern lives in the crate that owns it, next to the code. This file keeps
only what spans crates.

| Concern | Where |
|---|---|
| Wire protocol (frozen contract), payload formats, `Aligner` | [`shared/README.md`](shared/README.md) |
| Transport primitives, `Proxy`, dropped-frame accounting | [`networking/README.md`](networking/README.md) |
| Camera and microphone devices, hardware notes | [`devices/README.md`](devices/README.md) |
| Status/control endpoint, camera config selection, `DEVICE_NODE_*` variables | [`device-node/README.md`](device-node/README.md) |
| Fan-in, hub status endpoint, sync relay, `HUB_*` variables | [`control-hub/README.md`](control-hub/README.md) |
| Consumer API, `CLIENT_*` variables | [`access-client/README.md`](access-client/README.md) |
| Python bindings | [`python-client/README.md`](python-client/README.md) |
| Calibration (ChArUco detection, intrinsics) | [`calibration/README.md`](calibration/README.md) |
| Pre-processor (transforms, config, placement, performance) | [`preprocess/README.md`](preprocess/README.md) |
| Recorder (session layout, sidecar, `RECORD_*`) | [`recorder/README.md`](recorder/README.md) |
| Live sink (`v4l2loopback`, `LIVE_*`) | [`live-sink/README.md`](live-sink/README.md) |
| Real-hardware results | [`docs/hardware/`](docs/hardware/) |
| Historical review at commit `40c8af5` | [`docs/history/CODE_REVIEW.md`](docs/history/CODE_REVIEW.md) |

## 8. Build order and remaining work

Items 1–21, 23, 24, 27 and 28 are complete; 22, 25 and 26 are implemented (26: its measurement tooling) but await their hardware runs. Items 22–26 were added 2026-11-05 to
deliver images to downstream applications (§1); 27 and 28 were added 2026-11-06 from the README TODO and the
Python gaps. Items 19–21 were added 2026-10-30 from the first hardware run. Item numbers
are stable so older references still resolve. Each item's spec, acceptance criteria and checklist is in its
own file, kept in the crate that owns it (cross-cutting items, and items whose crate does not exist yet, are in [`docs/steps/`](docs/steps/)). §8.1 gives the order in which to
implement the open items.

| Item | Stage | Status | Spec file |
|---|---|---|---|
| 1 | 1 — Transport and hub foundation | ✅ | [step-01](networking/docs/step-01-fix-proxy.md) |
| 2 | 1 — Transport and hub foundation | ✅ | [step-02](control-hub/docs/step-02-control-hub-main.md) |
| 3 | 4 — Consumers and Python | ✅ | [step-03](access-client/docs/step-03-access-client-frames.md) |
| 4 | 2 — Producers | ✅ | [step-04](devices/docs/step-04-device-enumeration-filters.md) |
| 5 | 1 — Transport and hub foundation | ✅ | [step-05](networking/docs/step-05-receive-path-panics.md) |
| 6 | 2 — Producers | ✅ | [step-06](device-node/docs/step-06-device-status-endpoint.md) |
| 7 | 2 — Producers | ✅ | [step-07](device-node/docs/step-07-generic-device-node.md) |
| 8 | 2 — Producers | ✅ | [step-08](devices/docs/step-08-camera-buffers-format.md) |
| 9 | 1 — Transport and hub foundation | ✅ | [step-09](networking/docs/step-09-connection-stats.md) |
| 10 | 5 — Sync | ✅ | [step-10](control-hub/docs/step-10-time-alignment.md) |
| 11 | 4 — Consumers and Python | ✅ | [step-11](python-client/docs/step-11-pyo3-bindings.md) |
| 12 | 6 — Hardware validation | ✅ | [step-12](docs/steps/step-12-hardware-validation.md) |
| 13 | 3 — Hub fan-in and hub status | ✅ | [step-13](control-hub/docs/step-13-multi-producer-fan-in.md) |
| 14 | 2 — Producers | ✅ | [step-14](device-node/docs/step-14-remote-control.md) |
| 15 | 4 — Consumers and Python | ✅ | [step-15](python-client/docs/step-15-python-packaging-tests.md) |
| 16 | 3 — Hub fan-in and hub status | ✅ | [step-16](control-hub/docs/step-16-hub-status-endpoint.md) |
| 17 | 7 — Calibration | ✅ | [step-17](calibration/docs/step-17-calibration.md) |
| 18 | 8 — Windows microphone | ✅ | [step-18](devices/docs/step-18-windows-microphone.md) |
| 19 | 2 — Producers | ✅ | [step-19](device-node/docs/step-19-camera-config-negotiation.md) |
| 20 | 2 — Producers | ✅ | [step-20](devices/docs/step-20-microphone-enumeration-pipewire.md) |
| 21 | 2 — Producers | ✅ | [step-21](devices/docs/step-21-microphone-sample-rate-on-the-wire.md) |
| 22 | 7 — Calibration | ⚠️ | [step-22](calibration/docs/step-22-camera-extrinsics.md) |
| 23 | 9 — Delivery | ✅ | [step-23](preprocess/docs/step-23-preprocessor.md) |
| 24 | 9 — Delivery | ✅ | [step-24](recorder/docs/step-24-recorder.md) |
| 25 | 9 — Delivery | ⚠️ | [step-25](live-sink/docs/step-25-live-output.md) |
| 26 | 6 — Hardware validation | ⚠️ | [step-26](docs/steps/step-26-multi-camera-run.md) |
| 27 | 2 — Producers | ✅ | [step-27](device-node/docs/step-27-saved-config.md) |
| 28 | 4 — Consumers and Python | ✅ | [step-28](python-client/docs/step-28-python-aligned-and-processed.md) |

### 8.1 Order of remaining work

1. **Item 25 (live output), hardware run:** the `#[ignore]` round-trip test and a webcam run into `v4l2loopback`
   on a host with the module.
2. **Item 22 (extrinsics), hardware run:** two real cameras and a printed board; the recorder copies the result
   into sessions.
3. **Item 26 (multi-camera run):** needs two hosts and cameras; `scripts/multi-camera-run.sh` collects the numbers. It can run whenever they are available, and its
   numbers decide whether hub-side pre-processing works across hosts. Re-measure the raw path there too: the
   2026-10-30 bandwidth figure predates the MJPG payload fix.

An item is done only
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
  from the node. Decided 2026-10-30 (item 19). Encoding in a consumer, off the transport path, is allowed: the
  recorder (item 24) encodes uncompressed input and stream-copies MJPG. The hub's pre-processor output is
  uncompressed for the same reason (item 23).
- Network video protocols for third-party players (RTSP, HLS) — live use is through the ZMQ ports or a local
  `v4l2loopback` device (item 25).
- The downstream pose pipeline (segmentation, SMPL-X extraction, body-part classification, keypoint
  refinement) and any model training — a separate project that consumes this one's images (§1). Decided
  2026-11-05.
