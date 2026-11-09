# Step 26: Multi-camera and cross-host hardware run

- **Status:** ⚠️ Tooling done (2026-11-09), hardware run open: the hub reports throughput and CPU, the producers'
  part size is configurable, and one script collects every number below into a results note. The run itself needs
  two cameras on two hosts
- **Stage:** 6 — Hardware validation
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 26

## Scope

Step 12 validated one camera and one microphone on one host. Downstream use needs several cameras, usually on
several hosts. Measure, through the full path with sync on:

- two or more cameras on two or more producer hosts, the hub on another host, and a consumer;
- per camera: delivered rate, dropped frames, transport latency; per group: skew;
- hub CPU use and network throughput at 1080p MJPG, and with `YUYV` at the largest uncompressed mode.
- the effect of the producers' chunk size (`device-node` and `HUB_DATA_CHUNK_SIZE`, default 1 KiB). The
  pre-processor run found 1 KiB parts costly for multi-megabyte frames
  ([results](../hardware/2026-11-05-preprocessor.md)).

The results decide whether hub-side pre-processing (step 23) is usable across hosts, and set the
`HUB_SYNC_MAX_WAIT_MS` default for cross-host deployments.

Acceptance: a dated results note in `docs/hardware/` with the numbers above for at least two cameras on two
hosts for at least 60 s.

## Tooling (2026-11-09)

Everything the run measures is produced by the system itself, so the run is one command per host:

- **Hub `GET /status`** now also reports `uptime_s`, the forwarded stream's `frames_per_s` and `mbit_per_s` over
  the last second, the hub process's `cpu_percent` of one core (`getrusage`, all threads), and running `frames`
  and `bytes` totals (`control-hub/src/metrics.rs`: a counting subscriber on the proxy's internal endpoint plus a
  one-second sampler). The peer counts are unchanged.
- **Producer part size**: `DEVICE_NODE_DATA_CHUNK_SIZE` and `DEVICE_NODE_QUEUE_SIZE` (defaults 1024 and 128, read in
  both `run` modes, not part of a saved config) so the chunk-size comparison needs no rebuild.
- **`scripts/multi-camera-run.sh`**, run on the consumer host: reads the raw and aligned streams with
  `access-client` in summary mode for `RUN_SECS`, polls the hub's status every `STATUS_SECS`, and writes
  `docs/hardware/<date>-multi-camera-run.md` with a per-camera table (frames, rate, Mbit/s, latency, drops), an
  aligned table (groups/s, skew mean and max, group latency, frames per device), a hub table (peers, frames/s,
  Mbit/s, CPU mean and max), a conclusions section to fill in, and the raw logs. Configured by `HUB_IP`,
  `HUB_PORT`, `SYNC_PORT`, `HUB_STATUS_URL`, `RUN_SECS`, `LABEL`, `OUT` (see the script header).

Smoke-tested 2026-11-09 on one host with the laptop webcam at 640×480 YUYV: 30.0 fps, 147 Mbit/s, latency 7.6 ms
mean, 0 dropped, 30 groups/s, hub at 13 % of one core.

### Run recipe

```bash
# producer host A (repeat on host B with its camera and a different bind port if on one host)
DEVICE_NODE_DATA_CHUNK_SIZE=1024 device-node run camera 0 0.0.0.0 5555
# hub host
HUB_PRODUCERS=10.0.0.11:5555,10.0.0.12:5555 HUB_SYNC_PORT=5557 HUB_SYNC_DEVICES=/dev/video0,/dev/video0 \
  HUB_STATUS_ADDR=0.0.0.0:8081 control-hub
# consumer host
HUB_IP=10.0.0.2 HUB_STATUS_URL=http://10.0.0.2:8081/status LABEL="2 cameras, 1080p MJPG, 1 KiB parts" \
  scripts/multi-camera-run.sh
```

Repeat with `DEVICE_NODE_DATA_CHUNK_SIZE=4194304` (and `HUB_DATA_CHUNK_SIZE` to match) and with the YUYV mode, one
note per configuration or one note with several runs appended. Device ids repeat across hosts (`/dev/video0` on
both), so `HUB_SYNC_DEVICES` lists them per producer; the per-camera table then shows one row per distinct id, and
a run with identical ids needs one camera moved to `/dev/video1` or a per-host udev name before the numbers can be
told apart.

## Step checklist

- [x] Hub reports throughput and CPU on `/status`
- [x] Producer part size configurable without a rebuild (`DEVICE_NODE_DATA_CHUNK_SIZE`)
- [x] Measurement script writes the results note (`scripts/multi-camera-run.sh`, smoke-tested on one webcam)
- [ ] Two-camera, two-host run with sync on, at least 60 s
- [ ] Per-camera rate, drops and latency; group skew
- [ ] Hub CPU and network throughput recorded
- [ ] Conclusion recorded for steps 23 and 10
