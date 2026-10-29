# Step 21: Microphone sample rate on the wire

- **Status:** ❌ Open
- **Stage:** 2 — Producers (devices and device-node)
- **Spec item:** [`SPECS.md`](../../../SPECS.md) §8 item 21

## Why this step exists

Found during the 2026-10-30 microphone run ([results](../../hardware/2026-10-30-framework-12-webcam.md)). A
microphone frame carries `frame_shape = [buffer_size, channels]` and `S16LE` samples, but not the sample rate.
`AlsaMicrophoneDevice::start` asks ALSA for the configured rate with `ValueOr::Nearest`, so a card that cannot do
16 kHz streams at another rate and nothing on the wire says so. Consumers currently have to assume the node's
configured rate. Step 19 solved the same class of problem for cameras with `pixel_format`.

## Scope

1. Put the granted sample rate on the wire. Either add `sample_rate: Option<u32>` to `FramePacketInformation`
   next to `pixel_format`, or widen `frame_shape` to `[buffer_size, channels, sample_rate]`; the former keeps
   `frame_shape` meaning "shape" and is preferred.
2. `start()` reads the rate ALSA actually granted (`pcm.hw_params_current()`) and uses it for the header. If it
   differs from the requested rate, log a warning rather than fail, since the `plug` layer usually hides it.
3. Expose it in the Python `Frame` and document it in `SPECS.md` §7 and the READMEs.

Acceptance: the loopback pytest asserts the field, the networking round-trip test covers it, and the ignored
microphone test prints the header rate next to the measured one.

## Step checklist

- [ ] Sample rate in `FramePacketInformation`
- [ ] `start()` reads back the granted rate
- [ ] Python `Frame` exposes it; `SPECS.md` §7 updated
- [ ] Loopback pytest and round-trip test cover it
