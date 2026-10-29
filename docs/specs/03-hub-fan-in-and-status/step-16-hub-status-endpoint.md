# Step 16: Hub `GET /status` with producer and consumer counts

- **Status:** ✅ Done
- **Stage:** 3 — Hub fan-in and hub status
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 16

## Scope

`control-hub` serves `GET /status` with live producer and consumer counts. The counts come from ZMQ socket monitor events (`networking::PeerCounts`). The bind address is set by `HUB_STATUS_ADDR`, default `127.0.0.1:8081`.

## Evidence

- `control-hub/src/status.rs` exists.
- `networking/tests/peer-count-test.rs` exists.
- The status unit test in `control-hub/src/status.rs` queries the endpoint while one producer and one consumer are connected (`SPECS.md` §8 item 16).

## Step checklist

- [x] `control-hub/src/status.rs` serves `GET /status`
- [x] Peer counts come from socket monitor events (`networking/tests/peer-count-test.rs`)
- [x] Status unit test covers one producer and one consumer

## Stage 3 definition of done

- [x] `HUB_PRODUCERS` is parsed and the hub connects its XSUB socket to every endpoint
- [x] Two-producer loopback tests exist in `access-client/tests/`
- [x] Hub `GET /status` reports producer and consumer counts
- [ ] `cargo test --workspace` passes (not run in the last review)
