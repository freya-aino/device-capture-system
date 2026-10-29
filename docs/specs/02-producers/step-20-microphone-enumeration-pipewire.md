# Step 20: Microphone enumeration on ALSA hosts with PipeWire

- **Status:** ✅ Done (2026-10-30)
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 20

## Why this step exists

The 2026-10-30 hardware run ([results](../../hardware/2026-10-30-framework-12-webcam.md)) found that
`device-node list` reports zero microphones on a machine with one ALSA capture PCM. The filters added in step 04
skip `default`, `null`, `pipewire` and `sysdefault` by base name and then skip any hint containing `DEV`. On this
host the only remaining hardware hint is `front:CARD=PCH,DEV=0`, so the `DEV` filter removes it and nothing is left.

The step 04 filters were written against ALSA hint lists without a sound server. On a PipeWire desktop the hint
list is short and `hw:`/`front:` entries are the only direct-hardware entries; they are also usually busy because
PipeWire owns the card, which makes `pipewire` or `default` the device that actually captures.

## Scope (as implemented)

1. Replace the `DEV` substring filter with a rule that keeps one hint per card. Keep `front:CARD=…,DEV=0` or
   `hw:CARD=…,DEV=n` entries and drop only duplicate subdevice hints.
2. Offer the sound server device when present: list `pipewire` (or `default` when its description names a sound
   server) as a microphone so capture works on hosts where the hardware PCM is busy.
3. Add a hardware-free unit test for the filter that feeds it the hint list recorded in the hardware note and
   checks that at least `front:CARD=PCH,DEV=0` survives.
4. Run the ALSA capture loop on this host for a few seconds through `device-node run mic` and record the result in
   the hardware note.

Acceptance: `device-node list` shows at least one microphone on the Framework laptop, the filter unit test passes,
and `device-node run mic 0 <hub>` publishes frames.

## Step checklist

- [x] `DEV` substring filter replaced by a per-card rule (`select_capture_hints`: `sysdefault` > `plughw` > `front` > `hw`, one per `CARD=`)
- [x] Sound-server PCMs (`pipewire`, `pulse`, `jack`) listed as microphones when present
- [x] Hardware-free filter tests with the recorded hint list and a classic ALSA list (`devices/src/microphone_linux.rs`)
- [x] `device-node list` finds the ALC285 capture device (`sysdefault:CARD=PCH`) and `pipewire`
- [x] Capture loop delivers 16 kHz mono from both PCMs in the ignored `microphone-stream-test` ([results](../../hardware/2026-10-30-framework-12-webcam.md)); the `device-node run mic` end-to-end publish is part of step 12

## Stage 2 definition of done (extended)

See [step 19](step-19-camera-config-negotiation.md).
