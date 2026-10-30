# Step 18: Windows microphone

- **Status:** ✅ Done (2026-10-30: Windows recorded as unsupported)
- **Stage:** 8 — Windows microphone
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 18

## Scope

Either implement `devices/src/microphone_windows.rs`, or record in the README Requirements section that Windows is unsupported. Lowest priority.

Acceptance: one of the two options is written down. If Windows is implemented, the device tests pass on Windows.

## Decision

Windows is unsupported. The README Requirements section says so, and the empty `microphone_windows.rs` and its
`cfg(target_os = "windows")` wiring were removed, since a module that only exists on an OS the crate cannot
build for (the camera is v4l2) was misleading.

A port would need, in order: a WASAPI `shared::Device` for the microphone, a Media Foundation or DirectShow
camera backend, a `pixel_format` mapping for the formats those deliver, and CI on Windows. None of this is
planned.

## Evidence

- README Requirements: "Linux only … Windows and macOS are not supported and not planned".
- `devices/src/lib.rs` has no Windows module.

## Step checklist

- [x] Windows recorded as unsupported in the README Requirements section (implementation option not taken)
- [x] Not implemented, so no Windows test run applies
- [x] README, `AGENT.md` and `SPECS.md` updated in the same change

## Stage 8 definition of done

- [x] Windows recorded as unsupported in README Requirements
- [x] Not applicable (not implemented)
