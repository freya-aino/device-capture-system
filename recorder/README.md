# recorder

Consumer binary that records selected hub streams to disk: one video per camera, one WAV per microphone, and a
per-frame JSON-lines sidecar with the wire timestamps ([item 24](docs/step-24-recorder.md)). It reads the hub like
any other consumer and never talks to producers.

> **Status:** implemented and tested (loopback integration tests; 30 s webcam recording in
> [`docs/hardware/`](../docs/hardware/2026-11-09-recorder-saved-config.md)). `ffmpeg` must be on the `PATH`; the
> recorder refuses to start without it.

The full specification (variables, session layout, sidecar fields, encoding, drop accounting) is in
[`docs/step-24-recorder.md`](docs/step-24-recorder.md). In short:

```bash
RECORD_DEVICES=/dev/video0,sysdefault:CARD=PCH RECORD_SOURCE=aligned RECORD_SECS=60 cargo run -p recorder
# -> recordings/20261106T101500Z/{session.json, dev_video0.mkv, dev_video0.jsonl, sysdefault_CARD_PCH.wav, ...}
```

Align recorded streams on the sidecar's `capture_us`, never on frame index × frame rate.

How it works: one writer thread per device behind a 64-frame queue (a full queue drops and counts, the main thread
never blocks); cameras go through an `ffmpeg` child (`MJPG` stream-copied, uncompressed input encoded with
`RECORD_CODEC`, nominal rate measured from the first 30 frames), microphones to a `S16LE` WAV at the frame's sample
rate. `RECORD_SOURCE=aligned` reads `HUB_SYNC_PORT` and writes a group only when it holds every recorded device, so
the files line up row by row. Ctrl+C or `RECORD_SECS` ends the session; `session.json` is rewritten with the counts
and any writer error.
