# AGENT.md

Guidance for AI coding agents working in this repository. Read this together with
[README.md](README.md), which describes what the system does.

## Project at a glance

Rust (edition 2024) cargo workspace that captures camera (v4l2) and microphone (ALSA) data on
Linux and streams raw frames over ZeroMQ.

```
shared/         data model shared by everything (traits, FramePacket, stats) — no I/O
networking/     ZeroMQ transport: Connection, Sender (PUB), Receiver (SUB), Proxy (XSUB/XPUB)
devices/        V4lCameraDevice, AlsaMicrophoneDevice — implement shared::Device
device-node/    bin: runs on a capture machine (lib part: DeviceNode + helpers)
control-hub/    bin: central aggregator (stub)
access-client/  bin: consumer client (stub)
```

Dependency direction is strictly one-way:
`shared` ← `networking` / `devices` ← `device-node` / `control-hub` / `access-client`.
Never introduce a dependency that points the other way.

## Commands

```bash
cargo build --workspace
cargo check --workspace --all-targets
cargo test  --workspace
cargo test  -p networking           # hardware-free transport tests (Sender/Receiver, Proxy)
cargo run   -p device-node          # defaults to `list` (see README for the full CLI)
cargo fmt --all
```

System packages are required before anything compiles (`libasound2-dev`, `libzmq3-dev`,
`libv4l-dev`, `pkg-config`, plus `clang`/`libclang` for `bindgen`). If a build fails inside
`alsa-sys` or `zmq-sys` with a `pkg-config exited with status code 1` message, it is a missing
system library, **not** a code problem — do not try to "fix" it by editing Rust sources. If v4l2
bindgen fails with missing `libclang`, install the libclang dev package or set `LIBCLANG_PATH`.

## Conventions used in this codebase

- Error handling is `anyhow`: functions return `Result<T, Error>` with `anyhow::Error`.
  Build ad-hoc errors with `Error::msg(...)` or `anyhow!(...)`, matching nearby code.
- Preconditions are expressed with `assert!` carrying a descriptive message.
- Capture devices implement `shared::Device`. `start()` spawns a `std::thread`, returns its
  `JoinHandle<Result<(), Error>>`, and delivers data by calling the caller-supplied
  `Box<dyn Fn(FramePacket) -> Result<(), Error> + Send + 'static>` callback per frame.
- Stopping is cooperative: `stop()` sends `DeviceCommand::Stop` over a `flume` channel that the
  capture loop polls with `try_recv()`. Keep that pattern for new devices.
- Wire types that cross the network derive `bincode::{Encode, Decode}` and live in `shared`.
- Modules are private (`mod x;`) and re-exported flat (`pub use x::*;`) from each crate's
  `lib.rs`. Follow that when adding a module.
- Large blocks of commented-out exploratory code exist throughout (e.g. `devices/src/manager.rs`,
  `networking/src/primitives.rs`, most of `device-node/src/main.rs`). Treat it as the author's
  scratch space: do not delete it unless the task says so, and do not assume it works.

## Known rough edges

Worth knowing before you debug something — fix only if the task asks for it.

- `devices/src/microphone_windows.rs` is empty; only Linux is supported today.
- `devices/src/microphone_linux.rs` still has known issues: the enumeration filter is suspicious
  (`HintIter` direction handling), `frame_shape` is hard-coded to `[..., 2]`, and the byte buffer
  sizing ignores `channels`.
- `devices/src/camera.rs` allocates 128 mmap buffers per camera and doesn't validate the
  negotiated v4l2 format; camera enumeration also doesn't filter on capture capabilities.
- `ConnectionStats::update` divides by elapsed **whole** seconds, so `current_fps` can be
  infinite for fast streams.
- `networking/src/receiver.rs` `assert!`s and `unwrap()`s on peer-controlled data; malformed
  packets can panic the receiver.
- `networking/src/sender.rs::send` takes a `data_chunk_size` argument even though it is already
  configured on the `Sender`.
- `device-node/src/helper.rs::volume_from_audio_frame` says RMS but currently computes mean
  absolute amplitude.
- The capture loops `println!` per frame, which dominates runtime at high frame rates.

## Working rules

1. Keep changes minimal and local to the crate that owns the concern; put anything shared by two
   crates into `shared`.
2. Prefer `cargo check --workspace --all-targets` for a fast feedback loop.
3. New transport logic should be covered the way `networking/tests/networking-test.rs` does it:
   a hardware-free round trip over ZMQ (currently using `inproc://` endpoints).
4. Logic that needs a real camera or microphone cannot be tested in CI — keep it thin and push
   the testable parts into pure functions (see `volume_from_audio_frame` in
   `device-node/src/helper.rs`).
5. Do not commit unless explicitly asked.
