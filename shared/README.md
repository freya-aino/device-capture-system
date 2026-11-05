# shared

Data model used by every other crate: the `Device` trait, `FramePacket`, `FramePacketInformation`,
`ConnectionStats`, camera-config selection and the `Aligner`. It performs no I/O. Every other crate
depends on it and it depends on none of them (see `AGENT.md`).

Build items for this crate's concerns are tracked in [`SPECS.md`](../SPECS.md) §8; the connection
statistics step lives in [`networking/docs/`](../networking/docs/).

## Wire protocol (frozen contract)

Anything built on top, including the Python client, depends on this. A frame is one ZMQ multipart message:

| Part | Content |
|---|---|
| `0` | `bincode::encode_to_vec(FramePacketInformation, config::standard())` |
| `1` | Sender-local `u64` sequence number (little-endian) |
| `2..n` | Raw frame bytes, split into `data_chunk_size`-byte chunks (the receiver concatenates them; chunk boundaries carry no meaning) |

```rust
pub struct FramePacketInformation {
    pub device_info: DeviceInformation,   // id, name, device_type, device_status
    pub rx_timestamp: Option<SystemTime>, // set by receiver on arrival
    pub tx_timestamp: Option<SystemTime>, // set by sender just before send
    pub frame_shape: Vec<u32>,
    pub pixel_format: Option<String>,     // camera: negotiated FourCC ("YUYV", "MJPG", ...); microphone: None
    pub sample_rate: Option<u32>,         // microphone: granted rate in Hz; camera: None
    pub capture_timestamp: Option<SystemTime>, // when the device captured the frame; sync aligns on this
}
```

`pixel_format` (item 19), `sample_rate` (item 21) and `capture_timestamp` (item 10) were added 2026-10-30.

- `tx_timestamp` is the send time; `rx_timestamp - tx_timestamp` is the transport latency. The receive path
  records it into `ConnectionStats` (best-effort; frames without a `tx_timestamp` are dropped).
- `capture_timestamp` is what alignment uses (`FramePacketInformation::capture_time()`, falling back to
  `tx_timestamp`). The camera takes it from the driver's buffer timestamp; the microphone from the read time
  minus the frames still queued in ALSA. A microphone behind a sound server delivers several packets at once
  when its quantum fills, all with the same send time but distinct capture times.
- Every peer in a deployment must be built from the same revision. There is no protocol version negotiation,
  and a pre-item-19 peer cannot decode frames from a post-item-19 peer.

### Payloads by device type

- **Camera:** the v4l2 buffer exactly as the camera delivered it, in the FourCC named by `pixel_format`,
  `frame_shape = [width, height]`. Consumers must branch on `pixel_format`: `YUYV` is 2 bytes per pixel,
  `MJPG` is one JPEG image per frame.
- **Microphone:** interleaved `S16LE` samples as bytes, `frame_shape = [buffer_size, channels]`,
  `sample_rate` = the rate ALSA granted in Hz.

Frames are transported exactly as the device delivered them. Nothing in the hot path encodes or transcodes
(`SPECS.md` §9).

## Camera config selection

`shared::select_camera_config` and `select_camera_config_by_policy` pick one mode from a camera's advertised
list. `device-node` wires them to environment variables; see [`device-node/README.md`](../device-node/README.md).

## Aligner

`shared::Aligner` is generic over a payload. It groups frames from several devices by capture time, reports
skew and counts dropped samples. The pairing tolerance and the wait window (`max_wait`) are separate, and an
anchor is dropped as soon as every other device has moved past it. It is used by the hub's `AlignedRelay`
(`networking`) and by `access-client`'s `AlignedClient`. See
[`control-hub/docs/step-10-time-alignment.md`](../control-hub/docs/step-10-time-alignment.md).
