# Build specs

Each build item from `SPECS.md` §8 has its own file here, grouped into stages. Every step file has its
status, scope, evidence, a step checklist, and the definition of done for its stage.

Item numbers are stable, so `Step N` here is the same as `SPECS.md` §8 item N.

Status markers: ✅ done · ⚠️ partial · ❌ open.

| Stage | Folder | Steps | Status |
|---|---|---|---|
| 1 — Transport and hub foundation | [01-transport-and-hub](01-transport-and-hub/) | 1 ✅, 2 ✅, 5 ✅, 9 ✅ | Done |
| 2 — Producers (devices and device-node) | [02-producers](02-producers/) | 4 ✅, 6 ✅, 7 ✅, 8 ✅, 14 ✅ | Done in code; hardware not exercised |
| 3 — Hub fan-in and hub status | [03-hub-fan-in-and-status](03-hub-fan-in-and-status/) | 13 ✅, 16 ✅ | Done |
| 4 — Consumers and Python | [04-consumers-and-python](04-consumers-and-python/) | 3 ✅, 11 ✅, 15 ✅ | Done; clean-virtualenv pass not re-run |
| 5 — Sync | [05-sync](05-sync/) | 10 ⚠️ | Partial |
| 6 — Hardware validation | [06-hardware-validation](06-hardware-validation/) | 12 ❌ | Open |
| 7 — Calibration | [07-calibration](07-calibration/) | 17 ❌ | Open |
| 8 — Windows microphone | [08-windows-microphone](08-windows-microphone/) | 18 ❌ | Open |

Totals: 14 done, 1 partial (item 10), 3 open (items 12, 17, 18).

### Order of remaining work

Implement the open items in this order: 12 → 10 → 17 → 18 (`SPECS.md` §8.1). An item is done only when its
acceptance check passes with `cargo test --workspace` (plus the hardware run for item 12), and the README
TODO and the step file are updated in the same change.

### Source

The status reflects `SPECS.md` §6 and §8, a read of the source and test files, and the README. The last
review did not run `cargo test`, the pytest suite, or any hardware check. Those items stay unchecked
until they are run.
