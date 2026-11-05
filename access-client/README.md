# access-client

Consumer library and CLI. `AccessClient` (`connect`, `next_frame`, `run`, `disconnect`, `dropped_frames`)
subscribes to the hub and yields decoded frame packets; `AlignedClient` wraps it to yield aligned groups. The
Python bindings in [`python-client/`](../python-client/) are built on this crate, so Python and Rust share one
wire decoder.

## CLI

`cargo run -p access-client` prints one line per frame. With a summary window it prints one line per device
(rate, bandwidth, latency, drops).

| Variable | Default |
|---|---|
| `CLIENT_SUMMARY_SECS` | `0`; summary window in seconds |
| `CLIENT_RUN_SECS` | `0`; bounded run in seconds |
| `CLIENT_PREPROCESS_CONFIG` | unset; path of a pre-processor TOML file to run on the raw stream locally ([`preprocess/README.md`](../preprocess/README.md)). Devices without a pipeline pass through. Every frame is processed, so a pipeline slower than the camera builds up latency |
| `CLIENT_ALIGNED_PORT` | `0` (off); read aligned groups from the hub's `HUB_SYNC_PORT` instead of the raw stream |

Aligned reading is opt-in rather than the default, because single-device deployments, the Python bindings and
the existing tests consume the raw stream.

## Build items

[Step 3 (access-client frames)](docs/step-03-access-client-frames.md).
