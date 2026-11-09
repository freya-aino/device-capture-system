# Step 28: Python access to aligned groups and processed frames

- **Status:** ✅ Done (2026-11-09; pytest 3 passed against the loopback example)
- **Stage:** 4 — Consumers and Python
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 28

## Scope

The Python bindings read only the raw stream. Downstream applications are often Python, so they need the other
two hub outputs:

1. **Aligned groups.** `AlignedSystem(hub_ip, sync_port)`, an iterator like `DeviceSystem` that yields
   `Group(frames: list[Frame], skew_s: float)`. It wraps `networking::GroupReceiver`.
2. **Processed frames.** `DeviceSystem(hub_ip, 5558)` already receives them; what is missing is a way to get
   pixels. `Frame.image()` returns `(width, height, channels, bytes)` for `RGB3`, `GREY`, `YUYV` (converted to RGB)
   and `MJPG` (decoded), using `preprocess::decode_payload`, so Python shares the Rust decoder. It raises
   `ValueError` for microphone frames. No numpy dependency: `numpy.frombuffer(b, numpy.uint8).reshape(h, w, c)`
   is documented in the README.

## Acceptance

- The loopback pytest is extended: a group of two devices arrives through `AlignedSystem`, and `Frame.image()`
  of a YUYV frame returns the expected RGB bytes.

## Step checklist

- [x] `AlignedSystem(hub_ip, sync_port)` with `next_group(timeout_ms)`, iteration and `close()`; `Group.frames`,
      `Group.skew_s`, `len(group)`
- [x] `Frame.image()` -> `(width, height, channels, bytes)` via `preprocess::decode_payload`; `ValueError` for
      microphone frames and undecodable payloads
- [x] pytest extended (the loopback example now runs an `AlignedRelay` on 5557 with two cameras sharing a capture
      time; a YUYV black/white pair decodes to the expected RGB bytes); README API tables updated
