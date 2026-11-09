# Step 24: Recorder: selected streams to video files

- **Status:** ✅ Done (2026-11-09): loopback integration tests for the raw and aligned sources, and a 30 s webcam
  and microphone recording ([results](../../docs/hardware/2026-11-09-recorder-saved-config.md))
- **Stage:** 9 — Delivery
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 24

## Scope

A consumer binary, `recorder`, that writes the selected streams of a session to disk, so that applications can train
on them or process them offline. It is a consumer of the hub like any other; it never talks to producers.

### Input

| Variable | Default | Meaning |
|---|---|---|
| `RECORD_HUB_IP` | `127.0.0.1` | hub address |
| `RECORD_SOURCE` | `raw` | `raw` (hub port 5556), `aligned` (`HUB_SYNC_PORT`) or `processed` (`HUB_PREPROCESS_PORT`) |
| `RECORD_HUB_PORT` | per source: 5556, 5557, 5558 | override the port of the chosen source |
| `RECORD_DEVICES` | required | comma-separated device ids to record; frames from other devices are ignored |
| `RECORD_DIR` | `./recordings` | parent directory of sessions |
| `RECORD_SECS` | `0` | stop after N seconds; `0` records until Ctrl+C |
| `RECORD_CODEC` | `ffv1` | `ffv1` (lossless) or `h264`, for uncompressed input |
| `RECORD_CALIBRATION` | unset | calibration file (step 22) copied into the session |

With `aligned`, a group is written only when it contains every recorded device, so all files of a session have
the same frame count and line up row by row. With `raw` and `processed`, each device is recorded as it arrives.

**The recorder does not skip frames.** Unlike the hub's live relays, it writes every frame it receives. Each device
has its own writer thread behind a bounded queue (64 frames). If a writer falls behind and its queue is full, the
frame is dropped and counted; drops, and sequence gaps seen on the wire, are written to `session.json`. A
recording is never silently incomplete.

### Session layout

```
<RECORD_DIR>/<session-id>/
  session.json           written at start, rewritten at stop
  calibration.json       copy of RECORD_CALIBRATION, when set
  <file-stem>.mkv        one per camera
  <file-stem>.wav        one per microphone
  <file-stem>.jsonl      one line per written frame, in file order
```

- `session-id` is the UTC start time, `YYYYMMDDTHHMMSSZ`; a second session in the same second gets `-2`, `-3`, ….
- `file-stem` is the device id with every character outside `[A-Za-z0-9._-]` replaced by `_`, and leading `_`
  removed: `/dev/video0` → `dev_video0`. Two devices that map to the same stem get `-2`, `-3`, … in the order
  they first appear.

`session.json`:

```json
{
  "session": "20261106T101500Z",
  "revision": "<git describe of the recorder build>",
  "source": "aligned",
  "hub": "tcp://127.0.0.1:5557",
  "started": "2026-11-06T10:15:00.123456Z",
  "stopped": "2026-11-06T10:16:00.234567Z",
  "devices": [
    {
      "device_id": "/dev/video0", "file_stem": "dev_video0", "device_type": "Camera",
      "pixel_format": "MJPG", "frame_shape": [1920, 1080], "codec": "copy",
      "frames_written": 1800, "frames_dropped": 0, "sequence_gaps": 0
    }
  ]
}
```

Sidecar line (`<file-stem>.jsonl`), one per frame actually written:

```json
{"index":0,"sequence":1234,"capture_us":1793912345123456,"tx_us":1793912345130000,"rx_us":1793912345131000,"width":1920,"height":1080,"pixel_format":"MJPG"}
```

`index` is the frame's position in the video (or the packet's position in the WAV), `sequence` the wire sequence
number, timestamps are microseconds since the Unix epoch (`null` when the header has none). For microphones,
`width`/`height` are replaced by `"samples"` and `"channels"`. **Consumers align on `capture_us`**, never on
`index` × nominal frame rate: the container's rate cannot represent jitter or drops.

### Writing

- Camera: an `ffmpeg` child process per device, fed through stdin.
  - `MJPG`: `-f mjpeg -i pipe:0 -c:v copy` into Matroska. No re-encode, no quality loss.
  - `YUYV`, `RGB3`, `GREY`: `-f rawvideo -pix_fmt yuyv422|rgb24|gray -video_size WxH -i pipe:0`, encoded with
    `RECORD_CODEC`.
  - The nominal rate passed to `ffmpeg` is measured from the first 30 frames' capture times; it only affects
    players, not alignment.
- Microphone: a WAV writer, `S16LE`, at the frame's `sample_rate` and channel count.
- A change of `frame_shape` or `pixel_format` mid-session stops that device's writer with an error recorded in
  `session.json`; the other devices continue.
- `ffmpeg` is a runtime dependency of the recorder only, checked at start. The recorder refuses to start without
  it: device types are only known once frames arrive, so it cannot tell a microphone-only session apart in
  advance (decided 2026-11-09; the alternative, failing on the first camera frame, would leave the session
  incomplete).
- `session.json` carries one extra field beyond the example above, `incomplete_groups`, only for the `aligned`
  source and only when it is non-zero: the number of groups skipped because a recorded device was missing.
- A device in `RECORD_DEVICES` that never sends keeps `device_type` and `codec` as `"unknown"`, empty
  `frame_shape`, and an empty sidecar; no media file is created for it.

## Acceptance

- Unit tests: configuration, file stems, session ids, sidecar and `session.json` formats.
- Integration test: a loopback publisher sends a synthetic YUYV camera and a microphone for 3 s; the recording has
  as many video frames (`ffprobe -count_frames`) as sidecar lines, sidecar timestamps equal the published ones,
  and the WAV has the published sample count and rate. Skipped with a message when `ffmpeg` is not installed.
- A webcam recording noted in `docs/hardware/`.

## Step checklist

- [x] Crate scaffold with `RECORD_*` configuration, validated at start
- [x] File stems and session ids
- [x] Sidecar line and `session.json` types with their JSON format tested
- [x] `Receiver::last_sequence`, `AccessClient::last_sequence` and `FrameGroup::sequences` expose each frame's wire
      sequence number
- [x] Hub input for the three sources, including aligned-group recording (`src/input.rs`)
- [x] Per-device writer threads with bounded queues and drop counting (`src/writer.rs`)
- [x] `ffmpeg` writer: MJPG stream copy; `ffv1` / `h264` encode for uncompressed input (`src/ffmpeg.rs`)
- [x] WAV writer (`src/wav.rs`)
- [x] Integration tests (`tests/record-test.rs`: raw camera + microphone, aligned two-camera with an incomplete
      group skipped); webcam recording noted in
      [`docs/hardware/2026-11-09-recorder-saved-config.md`](../../docs/hardware/2026-11-09-recorder-saved-config.md)

## Evidence

- `cargo test -p recorder`: 18 unit tests, 2 integration tests (skipped with a message without `ffmpeg`).
- Webcam, 30 s, raw source: 901 MJPG frames stream-copied (`ffprobe -count_frames` 901, sidecar 901 lines, 0
  dropped, 0 gaps) and 935 microphone packets (478 720 samples, 16 kHz WAV). Details in the hardware note.
- The webcam run found that the camera published its whole mmap buffer (~4.1 MB per MJPG frame instead of ~440 KB);
  fixed in `devices` the same day (`bytesused`).
