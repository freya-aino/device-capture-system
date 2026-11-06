# recorder

Consumer binary that records selected hub streams to disk: one video per camera, one WAV per microphone, and a
per-frame JSON-lines sidecar with the wire timestamps ([item 24](docs/step-24-recorder.md)). It reads the hub like
any other consumer and never talks to producers.

> **Status: scaffold.** Configuration (`RECORD_*`), session ids, file stems, and the sidecar and `session.json`
> formats are implemented and tested. The binary validates its configuration, prints what it would record, and
> exits with "not implemented yet". Capture and writing are next.

The full specification (variables, session layout, sidecar fields, encoding, drop accounting) is in
[`docs/step-24-recorder.md`](docs/step-24-recorder.md). In short:

```bash
RECORD_DEVICES=/dev/video0,sysdefault:CARD=PCH RECORD_SOURCE=aligned RECORD_SECS=60 cargo run -p recorder
# -> recordings/20261106T101500Z/{session.json, dev_video0.mkv, dev_video0.jsonl, sysdefault_CARD_PCH.wav, ...}
```

Align recorded streams on the sidecar's `capture_us`, never on frame index × frame rate.
