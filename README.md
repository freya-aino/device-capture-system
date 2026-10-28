# Device Capture System

A Rust workspace for capturing synchronized video and audio from multiple machines and streaming
the raw frames over the network.

Cameras are captured through **v4l2**, microphones through **ALSA**, and every captured frame is
published as a chunked **ZeroMQ** multipart message. A central hub is intended to collect those
streams, keep them in sync, and expose them to clients.

> **Status: early development, no sync yet.** Implemented: the capture layer (`devices`),
> the transport layer (`networking`), a `device-node` that publishes one camera or microphone stream
> and serves `GET /status` plus remote `POST /start`, `/stop` and `/restart`, a `control-hub` that fans in one or more producer
> endpoints to any number of consumers (XSUB/XPUB proxy), an `access-client`, and Python bindings.
> Not implemented yet: sync/calibration, a hub HTTP API,
> and Python packaging. See [TODO](#todo).

## Architecture

```
┌──────────────────────┐         ┌──────────────────────┐         ┌──────────────────────┐
│ device-node (host A) │ ─ PUB ─►│ control-hub          │ ─ SUB ─►│ access-client        │
│ v4l2 cam / ALSA mic  │         │ XSUB ─► XPUB         │         │ or python-client     │
│ GET /status, POST    │         │ (N prod, no sync)    │         │                      │
└──────────────────────┘         └──────────────────────┘         └──────────────────────┘
```

Current implementation note: `control-hub` fans in every producer endpoint listed in `HUB_PRODUCERS`
(or the single `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT` pair) and forwards them to consumers. Sync,
calibration, and a hub HTTP API are planned (see [TODO](#todo) and `SPECS.md`). The only HTTP endpoints
today are on each `device-node`: `GET /status` and `POST /start`, `/stop` and `/restart`.

### Crates

| Crate           | Kind | Description                                                                                                     |
|-----------------|------|-----------------------------------------------------------------------------------------------------------------|
| `shared`        | lib  | Common data model: the `Device` / `DeviceConfig` traits, `DeviceInformation`, `FramePacket(Information)`, `ConnectionStatus`, `ConnectionStats`. Frame metadata is (de)serialized with `bincode`. |
| `devices`       | lib  | Concrete capture devices: `V4lCameraDevice` (v4l2 mmap streaming) and `AlsaMicrophoneDevice` (ALSA PCM capture). Each runs its capture loop on its own thread and pushes `FramePacket`s into a user callback. |
| `networking`    | lib  | ZeroMQ transport: `Connection` (socket lifecycle + stats), `Sender` (PUB), `Receiver` (SUB) and `Proxy` (XSUB↔XPUB forwarder). |
| `device-node`   | bin+lib  | Runs on each capture machine: lists devices and publishes one camera or microphone stream to a hub. Serves `GET /status` and `POST /start`, `/stop`, `/restart` (`DEVICE_NODE_STATUS_ADDR`). |
| `control-hub`   | bin  | Central process: runs the XSUB/XPUB ZMQ proxy, fanning in the producer endpoints from `HUB_PRODUCERS` (or the single `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT`), configured by `HUB_*` environment variables. Sync, calibration and an HTTP API are planned. |
| `access-client` | bin+lib  | Consumer-side client used to subscribe to the hub's aggregated stream (CLI prints received frames; library yields decoded frames). |
| `python-client` | lib  | Python bindings via PyO3 (`device_system` extension module) that wrap `access-client` to receive frames from a hub. Not packaged yet; see [`python-client/README.md`](python-client/README.md). |

### Frame transport

A frame is sent as a single ZMQ multipart message:

| Part    | Content                                                            |
|---------|--------------------------------------------------------------------|
| `0`     | `bincode`-encoded `FramePacketInformation` (device info, frame shape, tx timestamp) |
| `1`     | Sender-local `u64` sequence number (little-endian)                  |
| `2..n`  | The raw frame bytes, split into `data_chunk_size` chunks            |

The sender stamps `tx_timestamp` just before sending; the receiver stamps `rx_timestamp` on
arrival and re-joins the data chunks, which makes per-frame latency measurable. Subscribers
subscribe to all topics, and the high-water mark on both sides is set from `queue_size`.

The receiver tracks per-device sequence gaps and counts them as dropped frames, which helps detect
silent drops caused by PUB high-water marks (drops are only observable once a later frame arrives).

Payload formats:

- **Camera** – raw v4l2 buffer in the FourCC of the requested config (default `YUYV`),
  `frame_shape = [width, height]` from that config. If the driver negotiates a different width,
  height or FourCC than requested, `start()` returns an error instead of silently using the
  negotiated format (see `CODE_REVIEW.md` §4.7).
- **Microphone** – interleaved `S16LE` samples as bytes, `frame_shape = [buffer_size, channels]`.
  Use `FramePacket::decode_to_i16_le` to get `i16` samples back.

## Requirements

- Linux (the capture code is Linux-only; `device-node` asserts this at startup)
- Rust toolchain with **edition 2024** support (Rust 1.85+)
- System libraries — pick one:

  **Option A: [devenv](https://devenv.sh)** (recommended if you have Nix). This repo's
  `devenv.nix`/`devenv.yaml` pin `pkg-config`, `alsa-lib`, `zeromq`, `libv4l`, a Rust toolchain and
  `LIBCLANG_PATH` together, so `bindgen` always finds a `libclang` that's ABI-compatible with the
  rest of the shell:

  ```bash
  devenv shell -- cargo build --workspace
  # or, with direnv: `direnv allow` once, then `cargo build --workspace` works directly
  ```

  **Option B: system packages**

  ```bash
  # Debian / Ubuntu
  sudo apt install build-essential pkg-config libasound2-dev libzmq3-dev libv4l-dev clang libclang-dev

  # Fedora
  sudo dnf install gcc pkgconf-pkg-config alsa-lib-devel zeromq-devel libv4l-devel clang clang-devel

  # Arch
  sudo pacman -S base-devel pkgconf alsa-lib zeromq v4l-utils clang
  ```

  Without `alsa.pc` / `libzmq` headers on the `pkg-config` path the build fails in the
  `alsa-sys` / `zmq-sys` build scripts. The `v4l` dependency uses `bindgen` and needs a working
  `libclang` installation; if `bindgen` can't find it, set `LIBCLANG_PATH` to your libclang dir. On
  a system with more than one `libclang` build available (e.g. NixOS outside of `devenv`), make sure
  the one you point at matches the glibc your `cargo`/`rustc` actually links against, or `bindgen`
  fails with an `undefined symbol` error rather than a "not found" one.

## Build & Run

```bash
cargo build --workspace
cargo run -p device-node              # defaults to `list`
cargo run -p device-node -- list

# One-machine demo (3-process topology):
# - `device-node` PUB binds on 127.0.0.1:5555
# - `control-hub` XSUB connects to 127.0.0.1:5555 and XPUB binds on :5556
cargo run -p device-node -- run mic 0 127.0.0.1 5555
cargo run -p control-hub
cargo run -p access-client            # connects to :5556 and prints frames
```

For a multi-host setup, the `<hub_ip> <hub_port>` arguments of `device-node run` are the address the
producer's PUB socket **binds** to, not the hub's address. Bind on a local address (for example
`0.0.0.0 5555`) on the capture machine, and point the hub at that producer with `HUB_PRODUCER_IP` and
`HUB_PRODUCER_PORT`. Renaming these arguments is still open. To fan in several producers, set
`HUB_PRODUCERS` to a comma-separated list such as `10.0.0.2:5555,10.0.0.3:5555`; it takes precedence
over `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT`.

### Configuration

Environment variables read by the binaries:

| Variable | Binary | Default | Meaning |
|---|---|---|---|
| `DEVICE_NODE_STATUS_ADDR` | `device-node` | `127.0.0.1:8080` | Bind address of the status and control server |
| `HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` | `control-hub` | `127.0.0.1` / `5555` | Producer endpoint the hub's XSUB socket **connects** to |
| `HUB_PRODUCERS` | `control-hub` | unset | Comma-separated `ip:port` producer endpoints; overrides `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT` when set |
| `HUB_CONSUMER_IP` / `HUB_CONSUMER_PORT` | `control-hub` | `0.0.0.0` / `5556` | Address the hub's XPUB socket **binds** for consumers |
| `HUB_QUEUE_SIZE` | `control-hub` | `128` | Queue size passed to the proxy |
| `HUB_DATA_CHUNK_SIZE` | `control-hub` | `1024` | Data chunk size passed to the proxy |

`device-node` uses a queue size of `128` and a chunk size of `1024` (fixed in `device-node/src/main.rs`).

### Status endpoint

Each `device-node run` serves a status and control endpoint:

```bash
DEVICE_NODE_STATUS_ADDR=127.0.0.1:8080 cargo run -p device-node -- run mic 0 127.0.0.1 5555
curl http://127.0.0.1:8080/status
# {"device":"Running","connection":"Active"}
```

The values are the `Debug` names of `DeviceStatus` and `ConnectionStatus`. Other paths return 404.
`POST /start`, `POST /stop` and `POST /restart` change the device state and return the new status in the same JSON format. A transition the device cannot make (for example `/start` while `Running`, or any control of a node that was never started) returns `409` with an `error` message. A device that fails during control returns `500` with `{"error":"device control failed"}`. Other methods or paths return `404`.

## Tests

```bash
cargo test --workspace
cargo test -p networking      # send/receive + proxy forwarding
```

The transport-layer tests are hardware-free and exercise the `Sender`/`Receiver` round trip and
`Proxy` forwarding. Device capture tests require an actual camera/microphone.

Note: if `libzmq` is missing and Cargo falls back to building `zeromq-src`, test execution may
abort in restricted/sandboxed environments. Prefer installing the system `libzmq` dev package.

## Device Setup

- Cameras are discovered by globbing `/dev/video*`; the device id is the path and the name comes
  from the v4l2 capability `card` string. Available resolutions / frame intervals can be queried
  with `Device::get_configs`.
- Microphones are discovered through ALSA PCM hints, skipping playback-only and virtual entries
  (`default`, `null`, `sysdefault`, `pipewire`, …).
- Make sure your user can access the devices, e.g. it is in the `video` and `audio` groups.

Defaults used by `device-node` (see `device-node/src/helper.rs`):

| Device     | Default configuration                                   |
|------------|----------------------------------------------------------|
| Camera     | 1920x1080, 30 fps (`1/30`), FourCC `YUYV`                 |
| Microphone | 16 kHz, 1 channel, 512-sample buffer, `S16LE`             |

## Usage

Capturing from a microphone and publishing the frames:

```rust
use devices::AlsaMicrophoneDevice;
use device_node::DeviceNode;
use shared::MicrophoneConfig;
use std::net::Ipv4Addr;

let mic = AlsaMicrophoneDevice::get_all_alsa_microphones()?.remove(0);
let mut node = DeviceNode::new(mic, Ipv4Addr::new(0, 0, 0, 0), 5555, 128, 1024);

node.start(
    MicrophoneConfig { sample_rate: 16000, channels: 1, buffer_size: 512 },
    &zmq::Context::new(),
)?;
```

Receiving the frames on the other side:

```rust
use networking::Receiver;
use std::net::Ipv4Addr;

let mut receiver = Receiver::new(Ipv4Addr::new(127, 0, 0, 1), 5555, 128, 1024);
receiver.start(&zmq::Context::new())?;

loop {
    if let Some(packet) = receiver.receive(0)? {
        println!("{} bytes from {}", packet.data.len(), packet.frame_info.device_info.name);
    }
}
```

`receive(0)` blocks until a frame arrives. `receive(zmq::DONTWAIT)` returns `None` at once when no
frame is queued, so it cannot be used as a loop condition.

## Python client

The `python-client` crate builds a PyO3 extension module named `device_system`. See
[`python-client/README.md`](python-client/README.md) for build steps. Example against a running hub:

```python
from device_system import DeviceSystem

ds = DeviceSystem("127.0.0.1", 5556)
for frame in ds.get_frame():
    print(frame.device_id, frame.device_type, frame.frame_shape, len(frame.data))
```

## TODO

The ordered build plan is in `SPECS.md` §8.

- [x] Device Node CLI
  - [x] List devices
  - [x] Run a single device (camera/mic) and publish frames
  - [ ] Generate / Save Config
- [ ] Device Node HTTP Server
  - [x] Read-only `GET /status` (device and connection state)
  - [x] StartDevice / StopDevice / RestartDevice (`POST /start`, `/stop`, `/restart`)
- [x] `DeviceNode` support for cameras + microphones
- [ ] Capture hardware validation
  - [ ] Exercise the v4l2 camera capture loop on real hardware
  - [ ] Exercise the ALSA microphone capture loop on real hardware
  - [ ] Windows microphone support (`devices/src/microphone_windows.rs` is empty)
- [ ] Control Hub
  - [x] Load bind/connect config from env and run a ZMQ proxy
  - [x] Multi-producer fan-in (`HUB_PRODUCERS`, comma-separated `ip:port` list)
  - [ ] Sync
  - [ ] Calibrate
    - [ ] choose & implement general reference point detection model
    - [ ] transfer calibration code from python-opencv
  - [ ] HTTP API (not started)
- [ ] Client
  - [x] Minimal Rust access client (`access-client`) that receives frames from the hub
  - [x] Python client bindings (PyO3 extension in `python-client/`)
    - [x] `DeviceSystem(hub_ip, hub_port)`
    - [x] `DeviceSystem.get_frame() -> generator`
  - [ ] Package Python bindings (maturin / `pyproject.toml`)
  - [ ] Automated tests for the Python bindings

## License

MIT — see [LICENSE](LICENSE).
