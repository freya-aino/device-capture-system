# Build specs

Each build item from `SPECS.md` §8 has its own file here, grouped into stages. Every step file has its
status, scope, evidence, a step checklist, and the definition of done for its stage.

Item numbers are stable, so `Step N` here is the same as `SPECS.md` §8 item N.

Status markers: ✅ done · ⚠️ partial · ❌ open.

| Stage | Folder | Steps | Status |
|---|---|---|---|
| 1 — Transport and hub foundation | [01-transport-and-hub](01-transport-and-hub/) | 1 ✅, 2 ✅, 5 ✅, 9 ✅ | Done |
| 2 — Producers (devices and device-node) | [02-producers](02-producers/) | 4 ⚠️, 6 ✅, 7 ✅, 8 ✅, 14 ✅, 19 ❌, 20 ❌ | Hardware run 2026-10-30 found the camera default and the microphone filter block real devices |
| 3 — Hub fan-in and hub status | [03-hub-fan-in-and-status](03-hub-fan-in-and-status/) | 13 ✅, 16 ✅ | Done |
| 4 — Consumers and Python | [04-consumers-and-python](04-consumers-and-python/) | 3 ✅, 11 ✅, 15 ✅ | Done; clean-virtualenv pytest re-run 2026-10-30 |
| 5 — Sync | [05-sync](05-sync/) | 10 ⚠️ | Partial |
| 6 — Hardware validation | [06-hardware-validation](06-hardware-validation/) | 12 ⚠️ | Device side run 2026-10-30; end to end blocked by 19 and 20 |
| 7 — Calibration | [07-calibration](07-calibration/) | 17 ❌ | Open |
| 8 — Windows microphone | [08-windows-microphone](08-windows-microphone/) | 18 ❌ | Open |

Totals: 13 done, 3 partial (items 4, 10, 12), 4 open (items 17, 18, 19, 20).

### Order of remaining work

Implement the open items in this order: 19 → 20 → 12 → 10 → 17 → 18 (`SPECS.md` §8.1). An item is done only when its
acceptance check passes with `cargo test --workspace` (plus the hardware run for item 12), and the README
TODO and the step file are updated in the same change.

### Source

The status reflects `SPECS.md` §6 and §8, a read of the source and test files, and the README. On 2026-10-30
`cargo test --workspace` (13 test binaries) and the Python loopback pytest in a fresh venv both passed at `3c3d183`,
and the device side was run on a Framework Laptop 12 (see [`docs/hardware/`](../hardware/)). The end-to-end
hardware run and the 60 s acceptance run have not happened.
