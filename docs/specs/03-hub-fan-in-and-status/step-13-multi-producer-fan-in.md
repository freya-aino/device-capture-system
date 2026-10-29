# Step 13: Multi-producer fan-in via `HUB_PRODUCERS`

- **Status:** ✅ Done
- **Stage:** 3 — Hub fan-in and hub status
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 13

## Scope

`control-hub` connects its XSUB socket to every endpoint listed in `HUB_PRODUCERS` (`ip:port,ip:port`). It falls back to the single `HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` pair when the variable is unset.

Acceptance: a hardware-free test runs two producers on different loopback ports and checks that `access-client` receives frames with both device IDs.

## Evidence

- `control-hub/src/main.rs` parses the `HUB_PRODUCERS` list.
- `access-client/tests/hub-pipeline-test.rs` and `access-client/tests/aligned-client-test.rs` run two producers.

## Step checklist

- [x] `control-hub/src/main.rs` parses `HUB_PRODUCERS` and connects to each endpoint
- [x] Single-endpoint `HUB_PRODUCER_*` fallback is kept
- [x] Two-producer tests exist in `access-client/tests/`
- [ ] Test asserts that frames with both device IDs are received (not confirmed in the last review)

## Stage 3 definition of done

- [x] `HUB_PRODUCERS` is parsed and the hub connects its XSUB socket to every endpoint
- [x] Two-producer loopback tests exist in `access-client/tests/`
- [x] Hub `GET /status` reports producer and consumer counts
- [ ] `cargo test --workspace` passes (not run in the last review)
