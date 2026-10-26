# SPEC.md — Device Capture System

This document specifies the intended design of the system: the deployment topologies it must
support, the roles each component plays, the wire protocol, and the current implementation state
against that spec. It consolidates the architecture described in `README.md`/`AGENT.md` with the
findings in `CODE_REVIEW.md` (2026-10-25, reviewed commit `9c2ff25`).

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
is that the fan-in/fan-out proxy built on top of it doesn't work yet (§6.2), not that the
transport choice is wrong.

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

Either path is additive — `access-client/src/main.rs` is currently empty (`fn main() {}`), so
there is no existing consumer-side behavior to retrofit around. Recommendation: prefer (2) once
`access-client` has real logic worth sharing, since it removes the risk of the Python decoder
silently drifting from the Rust wire format after a `shared` change.

---

## 6. Current implementation state

Condensed from `CODE_REVIEW.md` (commit `9c2ff25`); see that file for full detail, line numbers,
and the fix the review already applied (the `devices` crate's broken module re-export, §4.1
there).

| Component | State | Blocks which role(s) |
|---|---|---|
| `shared` (data model) | ✅ Works. `ConnectionStats::update` math is wrong (whole-second FPS → `inf`, cumulative-byte "bitrate", mislabeled inter-frame gap as latency; real `tx`/`rx` timestamps go unused) | Observability for all roles |
| `networking::Connection`/`Sender`/`Receiver` | ✅ Point-to-point works, covered by a real loopback test. `close()` calls `unbind` on a `connect`ed socket and `.unwrap()`s it — panics. Receiver `.unwrap()`s/`assert!`s on attacker-/peer-controlled input | Producer, Consumer |
| `networking::Proxy` | ❌ Broken: `set_subscribe(&[])` is invalid on `XSUB` (`EINVAL` on `start()`); even if that's fixed, `start()` never calls `zmq::proxy()` or spawns a forwarding thread, so it forwards nothing | **Receiver + Accumulator — the hub's core function doesn't exist yet** |
| `devices::V4lCameraDevice` | ⚠️ Compiles, enumerates (no capability filtering — lists non-capture `/dev/video*` nodes as cameras). Allocates 128 mmap buffers (~512MB/cam, ~4s latency at 30fps; 3–4 is typical). `frame_shape` taken from requested config, not the negotiated format | Producer (camera) |
| `devices::AlsaMicrophoneDevice` | ⚠️ Compiles, enumerates — but the direction filter is inverted (keeps duplex-only hints, drops real capture devices) and the virtual-device filter does exact-string match against `sysdefault:CARD=PCH`-style ids, so it never matches. Net effect observed on review hardware: real mics dropped, virtual `sysdefault` kept. Buffer sizing also ignores channel count | Producer (mic) |
| `devices::microphone_windows` | ❌ Empty file | Producer on Windows |
| `devices::manager` | ⬜ Fully commented out | n/a (dead) |
| `device-node::DeviceNode` | ⚠️ Only implemented for `AlsaMicrophoneDevice` (concrete `impl` block, despite being generic over `T: Device`) — cameras cannot be published at all. No HTTP control API (start/stop/status) | Producer (camera unusable; no remote control) |
| `control-hub` | ❌ Stub — `main()` just sleeps 100s. No config loading, no `Proxy` wiring, no sync/calibration | **Receiver, Accumulator, Syncer — none exist as a runnable process** |
| `access-client` | ❌ Stub — `fn main() {}` | Consumer |
| `DeviceStatus` | ❌ Set to `Available` at construction and never updated again anywhere — the `assert!(status == Available)` guards in `start()` can never fire (so `start()` can be called twice, leaking the first capture thread), and any future `/status` endpoint would be meaningless | All roles' observability |

### 6.1 What already satisfies the topology requirement (§3)
The *shape* of the deployment flexibility is already real: `Sender::new`/`Receiver::new`/
`Proxy::new` take `Ipv4Addr` + `u16` with no hardcoded host assumptions, and nothing in
`device-node`/`control-hub`/`access-client` assumes colocation or separation. Once the stubs in
§6.2 are filled in, every topology in §3 is reachable purely through config.

### 6.2 What blocks it today
1. `networking::Proxy` doesn't forward data (the XSUB subscribe bug + missing `zmq::proxy()`
   call) — this is the single highest-priority fix, since every multi-producer topology (§3.2,
   §3.3, and even §3.1 with >1 device) depends on the hub actually aggregating.
2. `control-hub` has no logic at all — it needs to load config, open `Proxy`, and (per the
   README TODO) eventually run sync/calibration.
3. `access-client` has no logic at all — it needs to open a `Receiver` against the hub and expose
   frames to a caller (and, per §5, eventually a Python-facing API).
4. `DeviceNode` needs a camera-generic implementation (today hardcoded to
   `AlsaMicrophoneDevice`) before a producer host can publish both cameras and mics.

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

Carried over from `CODE_REVIEW.md` §6, reframed against the topology goal: nothing in §3 is
reachable until the hub exists, so that dominates the order.

1. Fix `Proxy` (XSUB subscribe bug + add the actual `zmq::proxy()` forwarding) — add a loopback
   proxy integration test analogous to `networking-test.rs`. This unblocks every multi-device
   topology.
2. Build minimal `control-hub` `main()`: load config (bind addresses for producers-in, port for
   consumers-out), construct and run `Proxy`.
3. Build minimal `access-client` `main()`: connect a `Receiver` to the hub, expose frames via a
   simple callback/generator API — this is the foundation §5's Python bindings sit on top of.
4. Fix microphone enumeration (inverted filter, virtual-device string match) and add camera
   capability filtering (`V4L2_CAP_VIDEO_CAPTURE`).
5. Remove panics on the network receive path (`unwrap()`/`assert!` on peer-controlled data in
   `Receiver::receive`, `FramePacketInformation::deserialize`).
6. Make `DeviceStatus` actually transition (`Available` → `Running` → `Stopped`), then build the
   planned `/status` HTTP endpoint on `device-node` on top of real state.
7. Generalize `DeviceNode` beyond `AlsaMicrophoneDevice` so producer hosts can publish cameras.
8. Reduce camera buffer count (128 → 3–4) and validate the negotiated v4l2 format instead of
   trusting the request.
9. Rewrite `ConnectionStats` using the existing `tx_timestamp`/`rx_timestamp` pair and `Instant`
   instead of `SystemTime` for interval math.
10. Add sync (time alignment across producers) and calibration to `control-hub`, per the README
    TODO — this is the one piece of §2's "Syncer" role with no code today.
11. Only after 1–3 are solid: build the Python client (§5).

---

## 9. Non-goals (explicitly out of scope per the design above)

- WebRTC, or any browser-interop transport — see §4.
- NAT traversal / cross-internet deployment — the system assumes a trusted LAN.
- Per-frame encryption by default — not required on a trusted LAN; revisit only if a specific
  threat model demands it, and prefer a transport-level option (CurveZMQ) over switching protocols.
- Video/audio codecs in the hot path — frames are transported raw to avoid encode/decode latency
  and to keep frame-accurate sync tractable.
