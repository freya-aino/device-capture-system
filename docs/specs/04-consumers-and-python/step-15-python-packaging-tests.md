# Step 15: Python packaging and loopback pytest

- **Status:** ✅ Done
- **Stage:** 4 — Consumers and Python
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 15

## Scope

Package the Python bindings with maturin (`pyproject.toml`), and cover them with a loopback pytest. The test runs `examples/loopback-publisher.rs` and receives frames through `DeviceSystem`.

## Evidence

- `python-client/pyproject.toml` builds the extension with maturin.
- `python-client/tests/test_loopback.py` exists.
- `SPECS.md` §8 item 15 records a clean-virtualenv install and a passing pytest. That run was not repeated in the last review.

## Step checklist

- [x] `python-client/pyproject.toml` builds the extension with maturin
- [x] `python-client/tests/test_loopback.py` exists and runs the loopback example
- [ ] Clean-virtualenv install and passing pytest re-run and recorded

## Stage 4 definition of done

- [x] `AccessClient` exposes connect, next frame, run and disconnect
- [x] PyO3 `device_system` module wraps `AccessClient`
- [x] maturin packaging and loopback pytest exist
- [ ] Clean-virtualenv install and pytest re-run (not run in the last review)
