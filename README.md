# Device Capture System

A Rust workspace for capturing synchronized video and audio from multiple machines and streaming
the raw frames over the network.

Cameras are captured through **v4l2**, microphones through **ALSA**, and every captured frame is
published as a chunked **ZeroMQ** multipart message. A central hub is intended to collect those
streams, keep them in sync, and expose them to clients.

> **Status: early development.** Implemented: the capture layer (`devices`), the transport layer
> (`networking`), a `device-node` that publishes one camera or microphone stream and serves `GET /status`
> plus remote `POST /start`, `/stop` and `/restart`, a `control-hub` that fans in one or more producer
> endpoints to any number of consumers (XSUB/XPUB proxy) and serves `GET /status` with live producer and
> consumer counts, an `access-client` (CLI and library, including client-side aligned groups), and
> installable Python bindings. Hub-side sync is opt-in (`HUB_SYNC_PORT`) and has not been validated on real
> devices. Not implemented yet: calibration and Windows microphone capture. See [TODO](#todo).

## Architecture

```
┌──────────────────────┐         ┌──────────────────────┐         ┌──────────────────────┐
│ device-node (host A) │ ─ PUB ─►│ control-hub          │ ─ SUB ─►│ access-client        │
│ v4l2 cam / ALSA mic  │         │ XSUB ─► XPUB         │         │ or python-client     │
│ GET /status, POST    │         │ (N prod, opt. sync)  │         │                      │
└──────────────────────┘         └──────────────────────┘         └──────────────────────┘
```

Current implementation note: `control-hub` fans in every producer endpoint listed in `HUB_PRODUCERS`
(or the single `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT` pair) and forwards the frames unaligned to consumers.
Opt-in sync (`HUB_SYNC_PORT`) additionally publishes time-aligned groups on a separate PUB port, and
calibration is planned (see [TODO](#todo) and `SPECS.md`). The hub serves `GET /status` on `HUB_STATUS_ADDR`;
each `device-node` serves `GET /status` and `POST /start`, `/stop` and `/restart`.

### Crates

| Crate           | Kind | Description                                                                                                     |
|-----------------|------|-----------------------------------------------------------------------------------------------------------------|
| `shared`        | lib  | Common data model: the `Device` / `DeviceConfig` traits, `DeviceInformation`, `FramePacket(Information)`, `ConnectionStatus`, `ConnectionStats`. Frame metadata is (de)serialized with `bincode`. |
| `devices`       | lib  | Concrete capture devices: `V4lCameraDevice` (v4l2 mmap streaming) and `AlsaMicrophoneDevice` (ALSA PCM capture). Each runs its capture loop on its own thread and pushes `FramePacket`s into a user callback. |
| `networking`    | lib  | ZeroMQ transport: `Connection` (socket lifecycle + stats), `Sender` (PUB), `Receiver` (SUB) and `Proxy` (XSUB↔XPUB forwarder). |
| `device-node`   | bin+lib  | Runs on each capture machine: lists devices and publishes one camera or microphone stream to a hub. Serves `GET /status` and `POST /start`, `/stop`, `/restart` (`DEVICE_NODE_STATUS_ADDR`). |
| `control-hub`   | bin  | Central process: runs the XSUB/XPUB ZMQ proxy, fanning in the producer endpoints from `HUB_PRODUCERS` (or the single `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT`), configured by `HUB_*` environment variables. Serves `GET /status` with producer and consumer counts. Opt-in sync (`HUB_SYNC_PORT`) publishes aligned groups; calibration is planned. |
| `access-client` | bin+lib  | Consumer-side client that subscribes to the hub's XPUB stream. The CLI prints received frames; the library provides `AccessClient` (decoded frames) and `AlignedClient` (groups frames from several devices by capture time). Configured by `HUB_IP`/`HUB_PORT` and `CLIENT_*` environment variables. |
| `python-client` | lib  | Python bindings via PyO3 (`device_system` extension module) that wrap `access-client` to receive frames from a hub. Installable with maturin (`pip install ./python-client`); see [`python-client/README.md`](python-client/README.md). |

### Frame transport

A frame is sent as a single ZMQ multipart message:

| Part    | Content                                                            |
|---------|--------------------------------------------------------------------|
| `0`     | `bincode`-encoded `FramePacketInformation` (device info, frame shape, pixel format, tx timestamp) |
| `1`     | Sender-local `u64` sequence number (little-endian)                  |
| `2..n`  | The raw frame bytes, split into `data_chunk_size` chunks            |

The sender stamps `tx_timestamp` just before sending; the receiver stamps `rx_timestamp` on
arrival and re-joins the data chunks, which makes per-frame latency measurable. Subscribers
subscribe to all topics, and the high-water mark on both sides is set from `queue_size`.

The receiver tracks per-device sequence gaps and counts them as dropped frames, which helps detect
silent drops caused by PUB high-water marks (drops are only observable once a later frame arrives).

Payload formats:

- **Camera** – the v4l2 buffer as the camera delivered it, in the FourCC named by the header's
  `pixel_format` (`"YUYV"`, `"MJPG"`, ...), `frame_shape = [width, height]`. `device-node` picks the
  mode from what the camera advertises (see Configuration); if the driver then negotiates a
  different width, height or FourCC, `start()` returns an error instead of silently using the
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
| `DEVICE_NODE_CAMERA_POLICY` | `device-node` | `nearest` | How to pick one of the camera's advertised modes: `nearest` (closest to the requested size and rate, uncompressed wins ties), `best-fps`, `best-resolution`, `best-average` |
| `DEVICE_NODE_CAMERA_WIDTH` / `_HEIGHT` / `_FPS` | `device-node` | `1920` / `1080` / `30` | Target for `nearest`. Pick a size the camera offers as `YUYV` (often 640x480) to get uncompressed frames |
| `HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` | `control-hub` | `127.0.0.1` / `5555` | Producer endpoint the hub's XSUB socket **connects** to |
| `HUB_PRODUCERS` | `control-hub` | unset | Comma-separated `ip:port` producer endpoints; overrides `HUB_PRODUCER_IP`/`HUB_PRODUCER_PORT` when set |
| `HUB_CONSUMER_IP` / `HUB_CONSUMER_PORT` | `control-hub` | `0.0.0.0` / `5556` | Address the hub's XPUB socket **binds** for consumers |
| `HUB_QUEUE_SIZE` | `control-hub` | `128` | Queue size passed to the proxy |
| `HUB_STATUS_ADDR` | `control-hub` | `127.0.0.1:8081` | Bind address of the hub's `GET /status` server |
| `HUB_SYNC_PORT` | `control-hub` | unset | Enables the aligned relay; it binds a PUB socket on `HUB_CONSUMER_IP`:this port |
| `HUB_SYNC_DEVICES` | `control-hub` | unset | Comma-separated device ids that must all appear in one aligned group (required with `HUB_SYNC_PORT`) |
| `HUB_SYNC_TOLERANCE_MS` | `control-hub` | `20` | Maximum capture-time distance between samples in one group |
| `HUB_DATA_CHUNK_SIZE` | `control-hub` | `1024` | Data chunk size passed to the proxy |
| `HUB_IP` / `HUB_PORT` | `access-client` | `127.0.0.1` / `5556` | Hub XPUB endpoint the CLI **connects** to |
| `CLIENT_QUEUE_SIZE` | `access-client` | `128` | Queue size for the client's SUB socket |
| `CLIENT_DATA_CHUNK_SIZE` | `access-client` | `1024` | Data chunk size expected by the client |
| `CLIENT_SUMMARY_SECS` | `access-client` | `0` | When > 0, print one `summary:` line per device every N seconds (rate, Mbit/s, latency mean/max, dropped frames) instead of a line per frame |
| `CLIENT_RUN_SECS` | `access-client` | `0` | When > 0, exit after N seconds with a final per-device total (used by the hardware validation run) |

`device-node` uses a queue size of `128` and a chunk size of `1024` (fixed in `device-node/src/main.rs`).

### Status endpoint

Each `device-node run` serves a status and control endpoint:

```bash
DEVICE_NODE_STATUS_ADDR=127.0.0.1:8080 cargo run -p device-node -- run mic 0 127.0.0.1 5555
curl http://127.0.0.1:8080/status
# {"device":"Running","connection":"Active"}
```

The values are the `Debug` names of `DeviceStatus` and `ConnectionStatus`. Other paths return 404.
`POST /start`, `POST /stop` and `POST /restart` change the device state and return the new status in the same JSON format. A transition the device cannot make (for example `/start` while `Running`, or `/stop` while `Stopped`) returns `409` with an `error` message. Starting a node that has no stored configuration, or a device that fails during control, returns `500` with `{"error":"device control failed"}`. Other methods or paths return `404`.

The control hub serves a read-only endpoint on `HUB_STATUS_ADDR`:

```bash
curl http://127.0.0.1:8081/status
# {"producers":2,"consumers":1}
```

`producers` counts the producer connections on the hub's XSUB side, and `consumers` counts the consumer connections on its XPUB side.

## Tests

```bash
cargo test --workspace
cargo test -p networking      # send/receive, proxy forwarding, aligned relay
cargo test -p access-client   # client-side receive and alignment against a local proxy
cargo test -p device-node    # node lifecycle and status endpoint
```

The transport-layer tests are hardware-free and exercise the `Sender`/`Receiver` round trip and
`Proxy` forwarding. Device capture tests require an actual camera/microphone.

The Python binding tests (`python-client/tests/test_loopback.py`) start a loopback publisher with
`cargo run`, so they need a Rust toolchain and the system libraries listed above:

```bash
pip install -e "./python-client[test]"
pytest python-client/tests
```

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
| Camera     | Requested 1920x1080 at 30 fps; the actual mode is the nearest the camera advertises, often `MJPG` at that size (`DEVICE_NODE_CAMERA_*`) |
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

### Component guides

#### device-node

Runs on each capture machine. Device indices are the positions reported by `device-node list`.

```bash
cargo run -p device-node -- list
cargo run -p device-node -- run camera 0 0.0.0.0 5555   # publish camera 0 on :5555, nearest to 1920x1080@30
DEVICE_NODE_CAMERA_WIDTH=640 DEVICE_NODE_CAMERA_HEIGHT=480 cargo run -p device-node -- run camera 0 0.0.0.0 5555  # uncompressed YUYV on most webcams
DEVICE_NODE_CAMERA_POLICY=best-fps cargo run -p device-node -- run camera 0 0.0.0.0 5555
DEVICE_NODE_STATUS_ADDR=127.0.0.1:8080 cargo run -p device-node -- run mic 0 0.0.0.0 5555
```

While it runs, control it over HTTP. `POST` requests change state and return the new status:

```bash
curl http://127.0.0.1:8080/status
curl -X POST http://127.0.0.1:8080/stop
curl -X POST http://127.0.0.1:8080/start
curl -X POST http://127.0.0.1:8080/restart
```

Status codes: `200` with the new status, `409` for an invalid transition (for example `/start` while
`Running`), `500` when the device fails or has no stored configuration, and `404` for other paths.
Press Ctrl+C to stop the process.

#### control-hub

Runs once per deployment and fans in every producer:

```bash
HUB_PRODUCERS=10.0.0.2:5555,10.0.0.3:5555 HUB_CONSUMER_PORT=5556 cargo run -p control-hub
curl http://127.0.0.1:8081/status
```

To also publish time-aligned groups, enable sync. `HUB_SYNC_DEVICES` lists the device ids that must all
appear in one group, and `HUB_SYNC_TOLERANCE_MS` sets the maximum capture-time distance inside a group:

```bash
HUB_PRODUCERS=10.0.0.2:5555,10.0.0.3:5555 \
HUB_SYNC_PORT=5557 HUB_SYNC_DEVICES=/dev/video0,hw:1 HUB_SYNC_TOLERANCE_MS=20 \
cargo run -p control-hub
```

The unaligned stream stays on `HUB_CONSUMER_PORT`; the aligned groups are on `HUB_SYNC_PORT`. Consumers
of the aligned stream decode it with `networking::GroupReceiver` (see below). Set `HUB_SYNC_DEVICES`
to the device ids your producers report (camera: the `/dev/videoN` path; microphone: the ALSA PCM name).
A mismatch means no groups are ever produced. The list is split on commas, so ids that contain a comma
(for example `hw:CARD=PCH,DEV=0`) cannot be listed here.

#### access-client

The CLI prints each frame it receives from the hub:

```bash
HUB_IP=127.0.0.1 HUB_PORT=5556 cargo run -p access-client
# prints one line per frame: device name, device type, frame shape and payload size
```

The library exposes the same stream to Rust code:

```rust
use std::time::Duration;
use access_client::AccessClient;
use std::net::Ipv4Addr;

let mut client = AccessClient::new(Ipv4Addr::new(127, 0, 0, 1), 5556, 128, 1024);
client.connect()?;
loop {
    if let Some(packet) = client.next_frame(Duration::from_secs(5))? {
        println!("{} bytes from {}", packet.data.len(), packet.frame_info.device_info.id);
    }
}
```

`next_frame(timeout)` returns `None` when no frame arrives in time. `run(callback)` calls the closure for each
frame until it returns `false`. Use `AccessClient::new_endpoint("tcp://host:port", ...)` for an explicit URL.

#### Aligned groups (Rust)

To get frames from several devices grouped by capture time, wrap a client in `AlignedClient`. Each group
contains one `FramePacket` per device in the `devices` list. Frames without a `tx_timestamp` are ignored.

```rust
use std::time::Duration;
use access_client::{AccessClient, AlignedClient};
use std::net::Ipv4Addr;

let client = AccessClient::new(Ipv4Addr::new(127, 0, 0, 1), 5556, 128, 1024);
let mut aligned = AlignedClient::new(
    client,
    vec!["/dev/video0".to_string(), "hw:1".to_string()],
    Duration::from_millis(20),
);
aligned.connect()?;
if let Some(group) = aligned.next_group(Duration::from_secs(5))? {
    println!("group of {} frames, dropped so far: {}", group.samples.len(), aligned.dropped());
}
```

To consume the hub's pre-aligned groups from `HUB_SYNC_PORT`, use `networking::GroupReceiver` instead:

```rust
use networking::GroupReceiver;

let mut groups = GroupReceiver::new_endpoint("tcp://127.0.0.1:5557", 128, 1024);
groups.start(&zmq::Context::new())?;
if let Some(group) = groups.receive(0)? {
    println!("{} frames, skew {:?}", group.frames.len(), group.skew);
}
```

## Python client

The `python-client` crate builds a PyO3 extension module named `device_system`. See
[`python-client/README.md`](python-client/README.md) for build steps. Example against a running hub:

```python
from device_system import DeviceSystem

ds = DeviceSystem("127.0.0.1", 5556)
for frame in ds.get_frame():
    print(frame.device_id, frame.device_type, frame.frame_shape, len(frame.data))
```

Python notes:

- `DeviceSystem` uses a fixed receive queue of `10` and chunk size `1024`; it does not read `CLIENT_*` variables.
- The bindings do not expose aligned groups. Frames from several producers arrive interleaved, so use `device_id` to separate them, or use the Rust `AlignedClient` for sync.
- Build with `pip install ./python-client` (maturin backend, needs a Rust toolchain). For a manual `.so` copy, see [`python-client/README.md`](python-client/README.md).

## TODO

The ordered build plan is in `SPECS.md` §8, and each build item's spec and checklist is in [`docs/specs/`](docs/specs/README.md).

- [x] Device Node CLI
  - [x] List devices
  - [x] Run a single device (camera/mic) and publish frames
  - [ ] Generate / Save Config
- [ ] Device Node HTTP Server
  - [x] Read-only `GET /status` (device and connection state)
  - [x] StartDevice / StopDevice / RestartDevice (`POST /start`, `/stop`, `/restart`)
- [x] `DeviceNode` support for cameras + microphones
- [ ] Capture hardware validation (2026-10-30 single-host run, see [`docs/hardware/`](docs/hardware/); cross-host run still open)
  - [x] Exercise the v4l2 camera capture loop on real hardware (30.0 fps over 70 s through hub and client, 0 dropped)
  - [x] Camera config selected from the advertised list, FourCC carried on the wire (`docs/specs` step 19)
  - [x] Microphone enumeration keeps hardware hints on PipeWire hosts (`docs/specs` step 20)
  - [x] Exercise the ALSA microphone capture loop on real hardware (16 kHz mono over 70 s through hub and client, 0 dropped)
  - [ ] Microphone sample rate carried on the wire (`docs/specs` step 21)
  - [ ] Windows microphone support (`devices/src/microphone_windows.rs` is empty)
- [ ] Control Hub
  - [x] Load bind/connect config from env and run a ZMQ proxy
  - [x] Multi-producer fan-in (`HUB_PRODUCERS`, comma-separated `ip:port` list)
  - [~] Sync: opt-in hub-side `AlignedRelay` on `HUB_SYNC_PORT` (not validated on real devices); `access-client::AlignedClient` aligns on the consumer side; the unaligned stream is unchanged
  - [ ] Calibrate
    - [ ] choose & implement general reference point detection model
    - [ ] transfer calibration code from python-opencv
  - [x] `GET /status` with live producer and consumer counts (`HUB_STATUS_ADDR`)
- [ ] Client
  - [x] Minimal Rust access client (`access-client`) that receives frames from the hub
  - [x] Python client bindings (PyO3 extension in `python-client/`)
    - [x] `DeviceSystem(hub_ip, hub_port)`
    - [x] `DeviceSystem.get_frame() -> generator`
  - [x] Package Python bindings (maturin / `pyproject.toml`)
  - [x] Automated tests for the Python bindings (`python-client/tests/test_loopback.py`)

## License

MIT — see [LICENSE](LICENSE).
