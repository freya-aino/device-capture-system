# device-node

Producer binary. It owns one hardware device (camera or microphone), runs its capture loop and publishes
`FramePacket`s. `DeviceNode<T: Device>` is generic, serves a status/control HTTP endpoint, and the same binary
runs in every deployment layout. CLI usage is in the top-level [`README.md`](../README.md).

## Status endpoint

`GET /status` on `DEVICE_NODE_STATUS_ADDR` (default `127.0.0.1:8080`) returns `200` with
`Content-Type: application/json`:

```json
{"device":"Running","connection":"Active"}
```

The values are the `Debug` names of `DeviceStatus` and `ConnectionStatus`. Other paths and methods return
`404`. A failed state read returns `500` with `{"error":"status unavailable"}`.

### Remote control

- `POST /start`, `POST /stop`, `POST /restart` transition the device and respond with the new status in the
  same JSON format.
- A transition the current `DeviceStatus` does not allow (`/start` while `Running`, `/stop` or `/restart`
  while not `Running`, or any control on a node that was never started) returns `409` with
  `{"error":"cannot <Action> while device is <Status>"}`.
- A device failure during control returns `500` with `{"error":"device control failed"}`.

The server validates transitions before calling the device, so a bad request cannot trigger the device's own
start/stop assertions.

## Camera config selection

`device-node run camera` never streams a fixed config. It enumerates the camera's advertised modes
(`Device::get_configs`) and picks one with `device_node::CameraSelection`:

| `DEVICE_NODE_CAMERA_POLICY` | Rule | Extra variables |
|---|---|---|
| `nearest` (default) | `shared::select_camera_config`: closest resolution, then closest frame rate; an uncompressed FourCC wins an exact tie | `DEVICE_NODE_CAMERA_WIDTH`, `_HEIGHT`, `_FPS` (defaults 1920, 1080, 30) |
| `best-fps` | `select_camera_config_by_policy(BestFps)` | none |
| `best-resolution` | `…(BestResolution)` | none |
| `best-average` | `…(BestAverage)` | none |

The node prints the selection and the chosen config at start. A camera that only offers a mode as MJPG is
streamed as MJPG with `pixel_format = "MJPG"`; the node never transcodes (`SPECS.md` §9). To get uncompressed
frames, ask for a mode the camera offers uncompressed (`DEVICE_NODE_CAMERA_WIDTH=640 …`).

## Configuration

| Variable | Default |
|---|---|
| `DEVICE_NODE_STATUS_ADDR` | `127.0.0.1:8080` |
| `DEVICE_NODE_CAMERA_POLICY` | `nearest` |
| `DEVICE_NODE_CAMERA_WIDTH` / `_HEIGHT` / `_FPS` | `1920` / `1080` / `30`, used by `nearest` |

Microphone default: 16 kHz, 1 channel, 512-sample buffer, `S16LE` (`src/helper.rs`).

## Build items

[Step 6 (status endpoint)](docs/step-06-device-status-endpoint.md),
[Step 7 (generic node)](docs/step-07-generic-device-node.md),
[Step 14 (remote control)](docs/step-14-remote-control.md),
[Step 19 (camera config negotiation)](docs/step-19-camera-config-negotiation.md).
