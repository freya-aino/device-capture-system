# networking

ZeroMQ transport primitives. They are parameterized purely by `Ipv4Addr` + `u16` and use plain TCP, so the
same code runs over loopback or a real NIC. That is what lets every deployment layout (`SPECS.md` §3) be
reached by configuration alone.

| Type | Role |
|---|---|
| `Connection` | Shared socket and stats plumbing |
| `Sender` | PUB side; stamps `tx_timestamp` and a sequence part, chunks the payload |
| `Receiver` | SUB side; validates peer input, drops malformed frames, counts gaps from the sequence part |
| `Proxy` | XSUB/XPUB; `start()` runs `zmq::proxy_steerable` on a forwarding thread, `stop()` ends it through a control socket |
| `AlignedRelay`, `GroupReceiver`, `FrameGroup` | Publish and read time-aligned groups (hub sync) |
| `FrameRelay` | Runs a per-frame transform on the producers' frames and publishes the results on its own port; processes the newest frame per device so latency stays bounded (hub pre-processing) |
| `PeerCounts` | Live producer and consumer counts behind the hub status endpoint |

The frame layout is the wire protocol in [`shared/README.md`](../shared/README.md).

## Why ZeroMQ

Raw ZeroMQ is the right choice for a lowest-latency LAN design and should not be replaced with WebRTC
(`SPECS.md` §4). The one real cost is that PUB silently drops frames once a subscriber's high-water mark is
hit. `Receiver` therefore counts sequence gaps and exposes `dropped_frames`.

## Tests

Hardware-free: `cargo test -p networking`. New transport logic gets a round-trip test over ZMQ the way
`tests/networking-test.rs` does it.

## Build items

[Step 1 (Proxy)](docs/step-01-fix-proxy.md), [Step 5 (receive-path panics)](docs/step-05-receive-path-panics.md),
[Step 9 (ConnectionStats)](docs/step-09-connection-stats.md).
