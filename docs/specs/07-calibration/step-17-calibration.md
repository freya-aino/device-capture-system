# Step 17: Calibration reference-point model and python-opencv port

- **Status:** ❌ Open
- **Stage:** 7 — Calibration
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 17

## Scope

Choose a general reference-point detection model, then port the python-opencv calibration code.

Acceptance: a deterministic test on a checked-in fixture image returns the expected reference points.

## Evidence

- No calibration or OpenCV code exists in the repository.
- The README calibration TODO entries are unchecked.

## Step checklist

- [ ] Reference-point detection model chosen
- [ ] Reference-point detection model implemented
- [ ] python-opencv calibration code ported
- [ ] Checked-in fixture image added
- [ ] Deterministic test returns the expected reference points on the fixture
- [ ] README calibration TODO entries checked off

## Stage 7 definition of done

- [ ] Reference-point detection model chosen and implemented
- [ ] python-opencv calibration code ported
- [ ] Checked-in fixture image with a deterministic test returning the expected reference points
