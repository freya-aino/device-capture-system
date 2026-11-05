# Step 26: Multi-camera and cross-host hardware run

- **Status:** ❌ Open
- **Stage:** 6 — Hardware validation
- **Spec item:** [`SPECS.md`](../../SPECS.md) §8 item 26

## Scope

Step 12 validated one camera and one microphone on one host. Downstream use needs several cameras, usually on
several hosts. Measure, through the full path with sync on:

- two or more cameras on two or more producer hosts, the hub on another host, and a consumer;
- per camera: delivered rate, dropped frames, transport latency; per group: skew;
- hub CPU use and network throughput at 1080p MJPG, and with `YUYV` at the largest uncompressed mode.

The results decide whether hub-side pre-processing (step 23) is usable across hosts, and set the
`HUB_SYNC_MAX_WAIT_MS` default for cross-host deployments.

Acceptance: a dated results note in `docs/hardware/` with the numbers above for at least two cameras on two
hosts for at least 60 s.

## Step checklist

- [ ] Two-camera, two-host run with sync on, at least 60 s
- [ ] Per-camera rate, drops and latency; group skew
- [ ] Hub CPU and network throughput recorded
- [ ] Conclusion recorded for steps 23 and 10
