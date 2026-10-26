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
control-hub/    bin: central aggregator (XSUB/XPUB proxy; sync/calibration planned)
access-client/  bin+lib: consumer client that subscribes to the hub and yields decoded frames
python-client/  PyO3 extension module (`device_system`): `DeviceSystem` + `Frame` for Python consumers
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

If this machine has Nix, prefix each command with `devenv shell -- ` (e.g.
`devenv shell -- cargo test --workspace`). The checked-in `devenv.nix`/`devenv.yaml` pin
`pkg-config`, `alsa-lib`, `zeromq`, `libv4l`, a Rust toolchain and `LIBCLANG_PATH` to one consistent
Nix closure, which sidesteps system packages entirely and avoids `bindgen`/`libclang` ABI mismatches
(see `CODE_REVIEW.md` §2 for a concrete example of that failure mode).

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
  commenting it out, and check `CODE_REVIEW.md` §5.3 before assuming a block is still there.

## Known rough edges

Worth knowing before you debug something — fix only if the task asks for it. `CODE_REVIEW.md` is the
living, authoritative list; this is only the two items that still require real hardware to close out.

- `devices/src/microphone_windows.rs` is empty; only Linux is supported today.
- Camera and microphone capture (opening a real device and reading frames) has never been exercised
  against real hardware on this machine — it compiles and is covered by hardware-free tests
  (enumeration, status transitions, error paths), but the actual capture loop is unverified. See
  `CODE_REVIEW.md` §7 readiness scores.

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
