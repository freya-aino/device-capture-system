# Step 18: Windows microphone

- **Status:** ❌ Open
- **Stage:** 8 — Windows microphone
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 18

## Scope

Either implement `devices/src/microphone_windows.rs`, or record in the README Requirements section that Windows is unsupported. Lowest priority.

Acceptance: one of the two options is written down. If Windows is implemented, the device tests pass on Windows.

## Evidence

- `devices/src/microphone_windows.rs` is empty (1 byte).
- The README says "not implemented yet" but does not state that Windows is unsupported.

## Step checklist

- [ ] `devices/src/microphone_windows.rs` implemented, or Windows recorded as unsupported in the README Requirements section
- [ ] If implemented, device tests pass on Windows
- [ ] README and `SPECS.md` updated in the same change

## Stage 8 definition of done

- [ ] `devices/src/microphone_windows.rs` implemented, or Windows recorded as unsupported in README Requirements
- [ ] If implemented, device tests pass on Windows
