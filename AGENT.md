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
control-hub/    bin: central hub (XSUB/XPUB proxy fanning in every `HUB_PRODUCERS` endpoint; sync)
access-client/  bin+lib: consumer client that subscribes to the hub and yields decoded frames
recorder/       bin+lib: records selected hub streams to video/WAV files with per-frame sidecars (scaffold)
live-sink/      bin+lib: writes one hub stream into a v4l2loopback camera (scaffold)
preprocess/     lib: per-camera image transforms from TOML (pure Rust); run by control-hub's FrameRelay or a consumer
calibration/    lib: ChArUco reference-point detection and intrinsics on the `opencv` crate; consumes frames, not used by the hub
python-client/  PyO3 extension module (`device_system`): `DeviceSystem` + `Frame` for Python consumers
```

Docs follow the code: each crate's `README.md` is its spec (config, endpoints, protocol) and its
`docs/step-NN-*.md` files are the build items it owns. `SPECS.md` holds only what spans crates and the item
index; cross-cutting items live in `docs/steps/`, real-hardware results in `docs/hardware/`, and the old review
in `docs/history/`. Update the owning crate's README when behavior changes.

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

If this machine has Nix, prefix each command with `devenv shell -- ` (e.g.
`devenv shell -- cargo test --workspace`). The checked-in `devenv.nix`/`devenv.yaml` pin
`pkg-config`, `alsa-lib`, `zeromq`, `libv4l`, a Rust toolchain and `LIBCLANG_PATH` to one consistent
Nix closure, which sidesteps system packages entirely and avoids `bindgen`/`libclang` ABI mismatches
(see `docs/history/CODE_REVIEW.md` §2 for a concrete example of that failure mode).

Without Nix, system packages are required before anything compiles (`libasound2-dev`, `libzmq3-dev`,
`libv4l-dev`, `pkg-config`, plus `clang`/`libclang` for `bindgen`). If a build fails inside
`alsa-sys` or `zmq-sys` with a `pkg-config exited with status code 1` message, it is a missing
system library, **not** a code problem — do not try to "fix" it by editing Rust sources. If v4l2
bindgen fails with missing `libclang`, install the libclang dev package or set `LIBCLANG_PATH`; if it
fails with an `undefined symbol` error instead of "not found", you likely have more than one
incompatible `libclang` build available and picked the wrong one.

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
- Library code (`networking`, `devices`, `device-node`'s status server) logs through `tracing`
  (`tracing::debug!`/`warn!`/`info!`), not `println!`/`eprintln!`. Binaries call
  `tracing_subscriber::fmt::init()` at the top of `main()`. User-facing CLI output (usage text,
  "press Ctrl+C to stop", frame-received lines) stays `println!` — it's program output for an
  interactive operator, not a log stream. Follow this split for new code.
- Commented-out exploratory code used to accumulate throughout the tree (e.g. the old
  `devices/src/manager.rs`, `networking/src/primitives.rs`). Those dead modules have since been
  deleted — don't recreate the pattern; delete exploratory code once it's superseded instead of
  commenting it out, and check `docs/history/CODE_REVIEW.md` §5.3 before assuming a block is still there.

## Known rough edges

Worth knowing before you debug something — fix only if the task asks for it. The current status is in
`SPECS.md` §6 and the README TODO; `docs/history/CODE_REVIEW.md` is a historical review at commit `40c8af5` and its
line numbers may be stale.

- Linux only, by decision (README Requirements, `SPECS.md` §8 step 18). There is no Windows module any more; do not
  add `cfg(target_os = "windows")` stubs.
- Capture is validated on one host only (a Framework Laptop 12 webcam and microphone, `docs/hardware/`). Several
  cameras and several hosts are untested (item 26). Tests that need a real device are `#[ignore]`.
- Pre-processing belongs to the hub, never to the producers: device nodes publish native frames, and the hub's
  relays read the proxy's internal `inproc://` stream rather than subscribing to producers.
- `recorder` and `live-sink` are scaffolds: their configuration is real, their binaries refuse to run past it.

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
