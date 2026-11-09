# 2026-11-09: Recorder, saved node config, and the MJPG payload fix on the Framework Laptop 12 webcam

Results for [step 24](../../recorder/docs/step-24-recorder.md) and [step 27](../../device-node/docs/step-27-saved-config.md).
One host, release builds, three processes plus the recorder:

```
device-node   config save node.toml run camera 0 127.0.0.1 5555     (default selection -> 1920x1080 MJPG 1/30)
device-node   run --config node.toml                                (status_addr 127.0.0.1:18080)
device-node   run mic 0 127.0.0.1 5554                              (sysdefault:CARD=PCH, DEVICE_NODE_STATUS_ADDR=127.0.0.1:18082)
control-hub   HUB_PRODUCERS=127.0.0.1:5555,127.0.0.1:5554 HUB_STATUS_ADDR=127.0.0.1:18081
recorder      RECORD_DEVICES=/dev/video0,sysdefault:CARD=PCH RECORD_SECS=30
access-client CLIENT_SUMMARY_SECS=10 CLIENT_RUN_SECS=30             (in parallel, raw port)
```

Ports 8080 and 8081 are taken by unrelated services on this laptop (a container runtime and a web app), so every
status address was set explicitly. The hub reported `{"producers":2,"consumers":0}` before the clients connected.

## Saved config (step 27)

`config save` wrote:

```toml
device_type = "camera"
device_id = "/dev/video0"
bind = "127.0.0.1:5555"
status_addr = "127.0.0.1:18080"

[camera]
width = 1920
height = 1080
fps = 30.0
pixel_format = "MJPG"
```

`run --config node.toml` printed `Config node.toml: camera 'Framework Laptop 12 Webcam Modu' with CameraConfig {
1920x1080 @ 1/30 s/frame (30.0 fps) MJPG }` and streamed that mode for the whole run, the same as the original
`run camera 0`. Two edited copies were refused: `fps = 25.0` with "the camera no longer advertises 1920x1080 @ 25
fps MJPG (10 modes advertised)", and `device_id = "/dev/video7"` with "device \"/dev/video7\" is not present;
available: /dev/video0".

## Recording (step 24), raw source, 30 s

| Device | Written | Dropped (writer backlog) | Sequence gaps | File | Check |
|---|---|---|---|---|---|
| `/dev/video0`, 1920×1080 MJPG, codec `copy` | 901 frames, 30.01 fps from the sidecar | 0 | 0 | `dev_video0.mkv`, 395 MB (~438 KB per frame) | `ffprobe -count_frames` = 901 = sidecar lines |
| `sysdefault:CARD=PCH`, 512-sample packets, `wav` | 935 packets, 478 720 samples | 0 | 0 | `sysdefault_CARD_PCH.wav`, 957 KB, `pcm_s16le` 16 000 Hz mono, 29.95 s | sidecar 935 lines |

Sidecar `rx_us - capture_us` for the camera: mean 38 ms, max 65 ms (this includes the node's send and the
recorder's 100 ms-timeout poll). Capture intervals 31.8–36.2 ms, mean 33.3 ms. The client reading the same stream in
parallel: 30.0 fps, 104.4–104.8 Mbit/s, latency mean 6.6–7.3 ms, 0 dropped; microphone 31.2 pkt/s, 0 dropped.

## MJPG payload fix (devices)

The first recording of the day produced a 3.7 GB file for 900 frames: 4.1 MB per frame, of which ~450 KB was the
JPEG and the rest stale bytes after the `EOI` marker. The camera sent its whole mmap buffer (`buf.to_vec()`), sized
for the largest possible frame, instead of the `bytesused` the driver reports. After slicing to `bytesused`:

| | Before | After |
|---|---|---|
| Bytes per 1080p MJPG frame | ~4.1 MB | ~438 KB |
| Raw stream on the wire | ~995 Mbit/s (the 2026-10-30 figure) | ~105 Mbit/s |
| Client latency mean (`rx - tx`) | 22–28 ms (2026-10-30), 64 ms sidecar `rx - capture` today | 6.6–7.3 ms; 38 ms sidecar `rx - capture` |
| 30 s recording | 3.7 GB | 395 MB |

Uncompressed modes are unaffected (`bytesused` equals the buffer size). The 2026-10-30 note's bandwidth statement
is corrected in place.

## Live sink (step 25)

Not run: this kernel does not ship `v4l2loopback` (`modinfo v4l2loopback` fails), and the module cannot be loaded
without a rebuild of the system configuration. The crate's loopback-hub tests pass; the `#[ignore]` round-trip test
and the webcam run remain for a host with the module.
