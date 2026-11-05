# Step 24: Recorder: selected streams to video files

- **Status:** ❌ Open
- **Stage:** 9 — Delivery
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 24

## Scope

A consumer binary, `recorder`, that writes the selected streams of a session to disk so that applications can
train on them or process them offline. When the crate exists, this file moves to `recorder/docs/`.

**Selection.** `RECORD_DEVICES` lists the device ids to record. `RECORD_SOURCE` is `raw` (default), `aligned`
(the hub's sync port) or `preprocessed` (the hub's pre-processor port, or a local `PREPROCESS_CONFIG`).
`RECORD_SECS` bounds the run; without it the recorder runs until stopped.

**Session layout** under `RECORD_DIR`:

```
<session-id>/
  session.json          devices, configs, source, start time, revision
  calibration.json      copied in when RECORD_CALIBRATION names one (step 22)
  <device-id>.mkv       one video per camera
  <device-id>.wav       one file per microphone
  <device-id>.jsonl     one line per frame: sequence, capture/tx/rx timestamps, frame_shape, pixel_format
```

The per-frame sidecar is the time reference. A container's nominal frame rate cannot represent dropped frames or
jitter, so consumers align on the sidecar's `capture_timestamp`, never on the frame index × fps.

**Encoding.** Encoding happens here, in a consumer, not in the transport path, so §9 still holds.
- `MJPG` input is stream-copied into the container, with no re-encode and no quality loss.
- `YUYV`, `RGB3` and `GREY` input is encoded with `RECORD_CODEC`: `ffv1` (lossless, default) or `h264`.
- Microphone frames are written as `S16LE` WAV at the granted `sample_rate`.

The proposed implementation pipes frames into an `ffmpeg` child process (`-c copy` for MJPG). That makes
`ffmpeg` a runtime dependency of the recorder only. The alternative is OpenCV's `VideoWriter`, which cannot
stream-copy MJPG.

Acceptance: a test that records a loopback publisher's synthetic camera and microphone streams for a few seconds
and checks that the video has as many frames as the sidecar has lines, that the sidecar timestamps match the
published ones, and that the WAV has the published sample count and rate. A real-camera recording is noted in
`docs/hardware/`.

## Step checklist

- [ ] `recorder` binary with `RECORD_*` configuration
- [ ] Session layout and `session.json`
- [ ] MJPG stream copy; `ffv1` / `h264` encode for uncompressed input
- [ ] Microphone to WAV
- [ ] Per-frame sidecar
- [ ] Loopback test as in the acceptance; real-camera recording noted
