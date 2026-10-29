# Step 01: Fix `Proxy` and add loopback proxy tests

- **Status:** ✅ Done
- **Stage:** 1 — Transport and hub foundation
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 1

## Scope

Make `networking::Proxy` forward frames between its XSUB and XPUB sides on a forwarding thread, and stop it cleanly through a control socket. Cover it with loopback tests.

## Evidence

- `Proxy::start()` runs `zmq::proxy_steerable` on a forwarding thread (`SPECS.md` §6).
- `networking/tests/proxy-test.rs` exists.
- `access-client/tests/hub-pipeline-test.rs` exists.

## Step checklist

- [x] `Proxy::start()` runs `proxy_steerable` on a forwarding thread
- [x] `Proxy::stop()` terminates the proxy through its control socket
- [x] Loopback proxy test exists (`networking/tests/proxy-test.rs`)
- [x] Hub pipeline test exists (`access-client/tests/hub-pipeline-test.rs`)

## Stage 1 definition of done

- [x] `Proxy` forwards frames between XSUB and XPUB and stops cleanly
- [x] Peer-controlled input on the receive path is dropped, not panicked on
- [x] `ConnectionStats` uses `Instant` and records per-frame latency
- [ ] `cargo test --workspace` passes (not run in the last review)
