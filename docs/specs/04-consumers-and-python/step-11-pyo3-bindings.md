# Step 11: PyO3 bindings in `python-client/`

- **Status:** ✅ Done
- **Stage:** 4 — Consumers and Python
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 11

## Scope

Expose the Python client as a PyO3 extension module named `device_system`. `DeviceSystem(hub_ip, hub_port)` with `get_frame()` and `next_frame()` wraps `access-client::AccessClient`, so Python uses the same Rust wire decoder.

## Evidence

- `python-client/src` contains the PyO3 bindings.
- `python-client/pyproject.toml` exists.
- `python-client/examples/loopback-publisher.rs` exists.

## Step checklist

- [x] PyO3 bindings exist in `python-client/src`
- [x] `DeviceSystem(hub_ip, hub_port)` wraps `AccessClient`
- [x] `get_frame()` and `next_frame()` are exposed
- [x] Loopback example exists (`python-client/examples/loopback-publisher.rs`)

## Stage 4 definition of done

- [x] `AccessClient` exposes connect, next frame, run and disconnect
- [x] PyO3 `device_system` module wraps `AccessClient`
- [x] maturin packaging and loopback pytest exist
- [x] Clean-virtualenv install and pytest re-run (2026-10-30: `pip install ./python-client` into a fresh venv, `pytest` 1 passed)
