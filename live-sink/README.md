# live-sink

Consumer binary that writes one selected hub stream into a `v4l2loopback` device, so applications that only open
a camera (OpenCV `VideoCapture`, OBS, browsers) can use it live ([item 25](docs/step-25-live-output.md)).
Applications built on this project's clients do not need it: they read the hub's raw, aligned and processed ports
directly.

> **Status:** implemented. Configuration (`LIVE_*`), the output-format decision (MJPG/YUYV pass through, RGB3/GREY
> become YUYV), the V4L2 writer and the newest-frame forwarding loop are tested against a loopback hub. The
> round trip through a real `v4l2loopback` device is an `#[ignore]` test
> (`cargo test -p live-sink -- --ignored`, `LIVE_TEST_SINK` overrides `/dev/video10`); it and the webcam run are
> still to be done on a host whose kernel ships the module.

```bash
sudo modprobe v4l2loopback video_nr=10 card_label="device-capture" exclusive_caps=1
LIVE_DEVICE=/dev/video0 LIVE_SINK=/dev/video10 LIVE_SOURCE=processed cargo run -p live-sink
ffplay /dev/video10
```

The sink has no transforms of its own: pre-processing lives with the hub (`LIVE_SOURCE=processed`).

The output format is fixed from the first frame of `LIVE_DEVICE`; a later frame with another size or format stops the
sink with an error. Frames that arrive while an older one is being written are skipped (counted and printed at exit),
so a slow reader never adds latency. On NixOS, `v4l2loopback` is a kernel module package
(`boot.extraModulePackages = [ config.boot.kernelPackages.v4l2loopback ];`).
