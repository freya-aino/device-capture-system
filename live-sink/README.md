# live-sink

Consumer binary that writes one selected hub stream into a `v4l2loopback` device, so applications that only open
a camera (OpenCV `VideoCapture`, OBS, browsers) can use it live ([item 25](docs/step-25-live-output.md)).
Applications built on this project's clients do not need it: they read the hub's raw, aligned and processed ports
directly.

> **Status: scaffold.** Configuration (`LIVE_*`) and the output-format decision (MJPG/YUYV pass through, RGB3/GREY
> become YUYV) are implemented and tested. The binary validates its configuration and exits with "not implemented
> yet". The V4L2 writer is next.

```bash
sudo modprobe v4l2loopback video_nr=10 card_label="device-capture" exclusive_caps=1
LIVE_DEVICE=/dev/video0 LIVE_SINK=/dev/video10 LIVE_SOURCE=processed cargo run -p live-sink
ffplay /dev/video10
```

The sink has no transforms of its own: pre-processing lives with the hub (`LIVE_SOURCE=processed`).
