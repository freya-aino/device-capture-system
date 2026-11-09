# Step 27: Generate and save a device-node configuration

- **Status:** ✅ Done (2026-11-09; unit tests and a webcam check, see Evidence)
- **Stage:** 2 — Producers
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 27

## Scope

The README TODO "Generate / Save Config". A capture host is set up once and should start the same way every time,
without retyping indices, addresses and `DEVICE_NODE_CAMERA_*` variables. Device indices from `device-node list`
are also unstable across reboots, so the saved file names devices by id (`/dev/video0`, `sysdefault:CARD=PCH`).

```bash
device-node config save node.toml run camera 0 0.0.0.0 5555   # resolve, write, do not start
device-node run --config node.toml                            # start from the file
```

`config save` takes the same arguments as `run`, resolves them as `run` would (camera mode selection included), and
writes:

```toml
device_type = "camera"            # or "microphone"
device_id = "/dev/video0"
bind = "0.0.0.0:5555"
status_addr = "127.0.0.1:8080"

[camera]                          # the resolved mode, not the policy
width = 1920
height = 1080
fps = 30
pixel_format = "MJPG"

# [microphone]
# sample_rate = 16000
# channels = 1
# buffer_size = 512
```

`run --config` looks the device up by `device_id`, and refuses to start if it is missing or no longer advertises
the saved mode. It never falls back to another device or mode silently. Environment variables do not override a
config file; a value in the file is the value used.

## Acceptance

- Unit tests: TOML round trip, rejection of unknown keys and missing sections, lookup by id among enumerated
  devices (with a fake device list), and the refusal when the saved mode is not advertised.
- On the webcam: `config save` then `run --config` streams the same mode as the original `run`.

## Step checklist

- [x] `config save <path> run …` writes the resolved configuration (`src/config.rs`, `NodeConfig`)
- [x] `run --config <path>` starts from it, by device id (`take_device_by_id`, `resolve_camera_mode`)
- [x] Missing device or mode is an error; environment variables are not consulted
- [x] Unit tests (TOML round trip, unknown keys, missing or doubled sections, lookup by id, unadvertised mode);
      webcam check; README usage

## Evidence

- `cargo test -p device-node`: 13 unit tests in the lib, including the five for `config.rs`.
- Webcam, 2026-11-09 ([note](../../docs/hardware/2026-11-09-recorder-saved-config.md)): `config save node.toml run
  camera 0 127.0.0.1 5555` wrote `/dev/video0`, 1920×1080 @ 30 fps `MJPG`; `run --config node.toml` streamed that
  mode for the 30 s recording. A copy with `fps = 25.0` was refused with "the camera no longer advertises 1920x1080 @
  25 fps MJPG (10 modes advertised)", and one naming `/dev/video7` with "device \"/dev/video7\" is not present;
  available: /dev/video0".
- `fps` is written as a float (`30.0`); integers in a hand-written file are accepted, and modes match within
  0.01 fps so `30000/1001` and `29.97` agree.
