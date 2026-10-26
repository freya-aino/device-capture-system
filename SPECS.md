# SPEC.md — Device Capture System

This document specifies the intended design of the system: the deployment topologies it must
support, the roles each component plays, the wire protocol, and the current implementation state
against that spec. It consolidates the architecture described in `README.md`/`AGENT.md` with the
findings in `CODE_REVIEW.md` (updated 2026-10-26, reviewed commit `eb9e36c`).

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
| **Accumulator** | Fans in frames from all connected producers into one logical stream | `control-hub` (via `networking::Proxy`'s XSUB→XPUB forwarding) |
| **Syncer** | Aligns frames from different producers/devices to a common clock/reference | `control-hub` (planned — see §6) |
| **Consumer** | Subscribes to the hub's aggregated output and does something with the frames | `access-client` bin, and/or third-party clients (e.g. Python) |

There is currently no separate "syncer" crate — sync is a planned responsibility *inside*
`control-hub`, not a standalone deployable unit. If a future requirement needs sync to scale or
fail independently of the hub's fan-in, it would become its own crate; nothing today requires
that split.

---

## 3. Deployment topologies

All topologies below must be reachable by changing only IP/port configuration — never by
recompiling or forking code paths. This is possible because `Sender`, `Receiver`, and `Proxy`
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

**Conclusion:** ZMQ is architecturally correct for this system's latency goal. The problem today
is that the fan-in/fan-out proxy built on top of it now works (§6.2). The open problems are
in sync, device capture and the receive path, not in the transport choice.

---

## 5. Third-party integration (Python et al.)

`README.md`'s TODO already specifies the target API:

```python
ds = DeviceSystem(hub_ip, hub_port)
for frame in ds.get_frame():
    ...
```

This is feasible without any FFI because the wire format is simple and fully specified in §7
below: a `pyzmq` `SUB` socket connected to the hub's XPUB port, decoding a small bincode header
(fixed field set, no dynamic schema) followed by concatenated raw bytes. Two implementation paths,
either is viable:

1. **Pure Python** — reimplement the bincode header decode (small, static format) with `pyzmq` +
   `struct`/a minimal bincode decoder. No Rust toolchain needed by consumers.
2. **PyO3 bindings** — wrap the real `networking::Receiver` (and `shared::FramePacket` decode) in
   a `pyo3` extension module, so the Python client shares the exact Rust implementation and never
   drifts from wire-format changes.

Either path is additive. `access-client` already holds the receive logic (`AccessClient` in
`access-client/src/lib.rs`), so a binding would wrap existing code. Recommendation: prefer (2),
since it removes the risk of the Python decoder silently drifting from the Rust wire format after a
`shared` change.

---

## 6. Current implementation state

Condensed from `CODE_REVIEW.md` (updated against commit `eb9e36c`); see that file for full detail,
line numbers, and the fix the review already applied (the `devices` crate's broken module
re-export, §4.1 there).

| Component | State | Blocks which role(s) |
|---|---|---|
| `shared` (data model) | ⚠️ Works, but `ConnectionStats::update` math is wrong (whole-second FPS → `inf`, cumulative-byte "bitrate", mislabeled inter-frame gap as latency; real `tx`/`rx` timestamps go unused) | Observability for all roles |
| `networking::Connection`/`Sender`/`Receiver` | ⚠️ Point-to-point works, covered by loopback tests. `close()` now uses `disconnect` and propagates errors. Receiver still `.unwrap()`s/`assert!`s on attacker-/peer-controlled input | Producer, Consumer |
| `networking::Proxy` | ✅ Works: `start()` runs `zmq::proxy_steerable` on a forwarding thread; `stop()` terminates it cleanly through a control socket. Covered by `proxy-test.rs` and the hub pipeline test | Receiver + Accumulator |
| `devices::V4lCameraDevice` | ⚠️ Compiles, enumerates (no capability filtering — lists non-capture `/dev/video*` nodes as cameras). Allocates 128 mmap buffers (~512MB/cam, ~4s latency at 30fps; 3–4 is typical). `frame_shape` taken from requested config, not the negotiated format | Producer (camera) |
| `devices::AlsaMicrophoneDevice` | ⚠️ Compiles, enumerates — but the direction filter is inverted (keeps duplex-only hints, drops real capture devices) and the virtual-device filter does exact-string match against `sysdefault:CARD=PCH`-style ids, so it never matches. Net effect observed on review hardware: real mics dropped, virtual `sysdefault` kept. Buffer sizing also ignores channel count | Producer (mic) |
| `devices::microphone_windows` | ❌ Empty file | Producer on Windows |
| `devices::manager` | ⬜ Fully commented out | n/a (dead) |
| `device-node::DeviceNode` | ✅ Generic over `T: Device`, so cameras and microphones use the same wiring. `status()` reports real device and connection state, served as JSON on `GET /status`. No remote start/stop control yet | Producer (status observable; no remote control) |
| `control-hub` | ⚠️ Runs `Proxy` from `HUB_*` environment config with Ctrl+C shutdown. No sync/calibration | Syncer (missing) |
| `access-client` | ✅ `AccessClient` (`connect`, `next_frame`, `run`, `disconnect`) plus a CLI binary. Covered by `access-client-test.rs` and the hub pipeline test. No Python binding | Consumer (Python API missing) |
| `DeviceStatus` | ❌ Set to `Available` at construction and never updated again anywhere — the `assert!(status == Available)` guards in `start()` can never fire (so `start()` can be called twice, leaking the first capture thread), and any future `/status` endpoint would be meaningless | All roles' observability |

### 6.1 What already satisfies the topology requirement (§3)
The *shape* of the deployment flexibility is already real: `Sender::new`/`Receiver::new`/
`Proxy::new` take `Ipv4Addr` + `u16` with no hardcoded host assumptions, and nothing in
`device-node`/`control-hub`/`access-client` assumes colocation or separation. Once the stubs in
§6.2 are wired up, the transport path of every topology in §3 is reachable through config alone.
Sync (§2 Syncer) is still missing.

### 6.2 What blocks it today
1. ✅ Resolved: `networking::Proxy` forwards data (`proxy_steerable`), and `control-hub` runs it.
2. ✅ Resolved for the transport path: `control-hub` loads config and runs `Proxy`. Sync and
   calibration (per the README TODO) are still missing.
3. ✅ Resolved: `access-client` exposes frames through `AccessClient`. A Python-facing API (§5) is
   still missing.
4. ✅ Resolved: `DeviceNode` is generic over `T: Device`. Its `DeviceStatus` still never changes
   (§6), so the `/status` endpoint is still blocked.

---

## 7. Wire protocol (frozen contract — anything built on top, including §5's Python client,
depends on this)

A frame is one ZMQ multipart message:

| Part | Content |
|---|---|
| `0` | `bincode::encode_to_vec(FramePacketInformation, config::standard())` |
| `1..n` | Raw frame bytes, split into `data_chunk_size`-byte chunks (receiver concatenates them back; chunk boundaries carry no meaning) |

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
  (currently hardcoded to `[buffer_size, 2]` regardless of actual channel count — see §6 table).

Per-frame latency is measurable today as `rx_timestamp - tx_timestamp`, but nothing in the
codebase currently computes it (`ConnectionStats::update` uses frame-to-frame gaps instead — §6).
Any consumer wanting real latency numbers must compute this itself from the two timestamps until
`ConnectionStats` is fixed.

Defaults (`device-node/src/helper.rs`):

| Device | Default config |
|---|---|
| Camera | 1920×1080, 30fps, `YUYV` |
| Microphone | 16kHz, 1 channel, 512-sample buffer, `S16LE` |

---

## 8. Recommended build order

Carried over from `CODE_REVIEW.md` §6, reframed against the topology goal. Items 1–3 and 7 are
done; the remaining work is the capture and receive-path fixes.

1. ✅ Done: fix `Proxy` and add loopback proxy tests (`proxy-test.rs`, hub pipeline test).
2. ✅ Done: minimal `control-hub` `main()` loads config and runs `Proxy`.
3. ✅ Done: `access-client` connects a `Receiver` to the hub and exposes frames via
   `AccessClient`. Still open: the Python bindings that sit on top of it (§5).
4. ✅ Done: microphone enumeration filter fixed (playback-only skipped, virtual devices matched by
   base name) and camera capability filtering (`V4L2_CAP_VIDEO_CAPTURE`) added. Build-verified
   with a Nix shell providing libclang; the hardware itself has not been exercised.
5. Remove panics on the network receive path (`unwrap()`/`assert!` on peer-controlled data in
   `Receiver::receive`, `FramePacketInformation::deserialize`).
6. ✅ Done: camera and microphone set `DeviceStatus` to `Running` on start and `Stopped` on stop,
   and `device-node run` serves `GET /status` (JSON with device and connection state) on
   `DEVICE_NODE_STATUS_ADDR` (default `127.0.0.1:8080`). Covered by `device-node/tests/status-test.rs`.
7. ✅ Done: `DeviceNode` is generic over `T: Device`, so producer hosts can publish cameras.
8. ✅ Partly done: camera buffer count reduced from 128 to 4. Still open: validating the
   negotiated v4l2 format instead of trusting the request.
9. Rewrite `ConnectionStats` using the existing `tx_timestamp`/`rx_timestamp` pair and `Instant`
   instead of `SystemTime` for interval math.
10. Add sync (time alignment across producers) and calibration to `control-hub`, per the README
    TODO — this is the one piece of §2's "Syncer" role with no code today.
11. ✅ Done: Python client (§5) as a PyO3 extension in `python-client/`. `DeviceSystem(hub_ip, hub_port)`
    with `get_frame()` / `next_frame()` wraps `AccessClient`, so Python shares the Rust wire decoder.
    Verified end to end against the loopback publisher example. Not packaged yet (no maturin config).

---

## 9. Non-goals (explicitly out of scope per the design above)

- WebRTC, or any browser-interop transport — see §4.
- NAT traversal / cross-internet deployment — the system assumes a trusted LAN.
- Per-frame encryption by default — not required on a trusted LAN; revisit only if a specific
  threat model demands it, and prefer a transport-level option (CurveZMQ) over switching protocols.
- Video/audio codecs in the hot path — frames are transported raw to avoid encode/decode latency
  and to keep frame-accurate sync tractable.
