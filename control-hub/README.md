# control-hub

Central hub binary. It is the Receiver, Accumulator and Syncer roles of `SPECS.md` §2: an XSUB/XPUB proxy that
connects to every producer endpoint and republishes the combined stream to consumers. It forwards frames
unchanged; sync is an opt-in extra output.

## Fan-in

The hub connects its XSUB socket to every endpoint in `HUB_PRODUCERS` (`ip:port,ip:port`), and falls back to
`HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` when it is unset. Several producer hosts fan in to one hub with no code
change.

## Status endpoint

`GET /status` on `HUB_STATUS_ADDR` returns live producer and consumer counts:

```json
{"producers":2,"consumers":1}
```

## Sync (opt-in)

When `HUB_SYNC_PORT` is set, an `AlignedRelay` aligns frames from the devices in `HUB_SYNC_DEVICES` by capture
time and publishes the groups on that separate PUB port. The raw forwarding stays unchanged. Consumers read
groups with `CLIENT_ALIGNED_PORT`, `networking::GroupReceiver`, or align themselves with `AlignedClient`.

Validated on real devices 2026-10-30: one group per camera frame at 30/s, skew ≤ 18 ms. Calibration is not part of the hub; it runs as a
consumer ([`calibration/`](../calibration/README.md), [item 17](../calibration/docs/step-17-calibration.md)).

## Configuration

| Variable | Default |
|---|---|
| `HUB_PRODUCER_IP` / `HUB_PRODUCER_PORT` | `127.0.0.1` / `5555` |
| `HUB_PRODUCERS` | unset; comma-separated `ip:port` list, takes precedence over the pair above |
| `HUB_CONSUMER_IP` / `HUB_CONSUMER_PORT` | `0.0.0.0` / `5556` |
| `HUB_QUEUE_SIZE` | `128` |
| `HUB_DATA_CHUNK_SIZE` | `1024` |
| `HUB_STATUS_ADDR` | `127.0.0.1:8081` |
| `HUB_SYNC_PORT` | unset (sync off); when set, `HUB_SYNC_DEVICES` is required |
| `HUB_SYNC_DEVICES` | unset; device ids to align |
| `HUB_SYNC_TOLERANCE_MS` | `20`; maximum capture-time distance inside one group |
| `HUB_SYNC_MAX_WAIT_MS` | `200`; how long a frame waits for partners before it is dropped (must cover the transport-latency difference between devices) |

## Build items

[Step 2 (hub main)](docs/step-02-control-hub-main.md),
[Step 10 (time alignment)](docs/step-10-time-alignment.md),
[Step 13 (multi-producer fan-in)](docs/step-13-multi-producer-fan-in.md),
[Step 16 (status endpoint)](docs/step-16-hub-status-endpoint.md).
