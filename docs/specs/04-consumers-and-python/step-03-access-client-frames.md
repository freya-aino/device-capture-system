# Step 03: `access-client` exposes frames via `AccessClient`

- **Status:** ✅ Done
- **Stage:** 4 — Consumers and Python
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 3

## Scope

`access-client` connects a `Receiver` to the hub and exposes the received frames through `AccessClient`, with `connect`, `next_frame`, `run` and `disconnect`. The Python bindings built on top of it are tracked as item 11.

## Evidence

- `access-client/src` exposes `AccessClient` (confirmed in the last review).
- The README Rust section documents `next_frame(timeout)`, `run(callback)` and `new_endpoint`.

## Step checklist

- [x] `AccessClient` exposes `connect`, `next_frame`, `run` and `disconnect` (`access-client/src`)
- [x] `access-client` connects a `Receiver` to the hub
- [x] Stale "Python packaging is open" wording removed from `SPECS.md` §8 item 3

## Stage 4 definition of done

- [x] `AccessClient` exposes connect, next frame, run and disconnect
- [x] PyO3 `device_system` module wraps `AccessClient`
- [x] maturin packaging and loopback pytest exist
- [x] Clean-virtualenv install and pytest re-run (2026-10-30: `pip install ./python-client` into a fresh venv, `pytest` 1 passed)
