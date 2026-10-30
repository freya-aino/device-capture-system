use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use devices::AlsaMicrophoneDevice;
use shared::{Device, DeviceStatus, FramePacket, MicrophoneConfig};

const CONFIG: MicrophoneConfig = MicrophoneConfig {
    sample_rate: 16000,
    channels: 1,
    buffer_size: 512,
};

/// Captures `seconds` of audio from one enumerated microphone and reports the delivered sample
/// rate and RMS level, so a silent or mis-rated capture is visible.
fn capture_and_report(mic: &mut AlsaMicrophoneDevice, seconds: u64) -> usize {
    println!("capturing from {:?} ({:?})", mic.id(), mic.name());
    let packets: Arc<Mutex<Vec<(Instant, FramePacket)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = packets.clone();
    let started = Instant::now();
    let handle = mic
        .start(
            CONFIG.clone(),
            Box::new(move |packet| {
                sink.lock().unwrap().push((Instant::now(), packet));
                Ok(())
            }),
        )
        .unwrap();
    sleep(Duration::from_secs(seconds));
    mic.stop().unwrap();
    let joined = handle.join().unwrap();
    assert_eq!(mic.device_status(), &DeviceStatus::Stopped);
    if let Err(e) = joined {
        println!("  capture thread failed: {e:#}");
        return 0;
    }

    let packets = packets.lock().unwrap();
    let count = packets.len();
    if let (Some((first, _)), Some((last, _))) = (packets.first(), packets.last()) {
        let span = last.duration_since(*first).as_secs_f64();
        let samples: usize = packets.iter().map(|(_, p)| p.data.len() / 2).sum();
        let frames_per_packet = CONFIG.buffer_size as usize * CONFIG.channels as usize;
        let delivered_rate = if span > 0.0 {
            (samples - frames_per_packet) as f64 / span / CONFIG.channels as f64
        } else {
            0.0
        };
        let all: Vec<i16> = packets
            .iter()
            .flat_map(|(_, p)| FramePacket::decode_to_i16_le(&p.data).unwrap())
            .collect();
        let rms = (all.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / all.len() as f64).sqrt()
            / i16::MAX as f64;
        let mean = all.iter().map(|s| *s as f64).sum::<f64>() / all.len() as f64;
        let min = all.iter().copied().min().unwrap();
        let max = all.iter().copied().max().unwrap();
        let clipped = all
            .iter()
            .filter(|s| **s == i16::MIN || **s == i16::MAX)
            .count();
        println!(
            "  samples: mean {mean:.1} min {min} max {max} clipped {clipped}; first 12 of packet 10: {:?}",
            &FramePacket::decode_to_i16_le(&packets[10.min(count - 1)].1.data).unwrap()[..12]
        );
        println!(
            "  first packet after {:.0} ms; {} packets, {} samples over {:.2} s = {:.0} Hz (expected {}, header says {:?}); rms {:.4}; shape {:?}",
            first.duration_since(started).as_secs_f64() * 1000.0,
            count,
            samples,
            span,
            delivered_rate,
            CONFIG.sample_rate,
            packets[0].1.frame_info.sample_rate,
            rms,
            packets[0].1.frame_info.frame_shape
        );
    } else {
        println!("  no packets");
    }
    count
}

#[test]
#[ignore = "requires an ALSA capture device"]
fn real_microphones_capture_for_a_few_seconds() {
    let mics = AlsaMicrophoneDevice::get_all_alsa_microphones().unwrap();
    assert!(!mics.is_empty(), "no microphones enumerated");
    let mut any = false;
    for mut mic in mics {
        any |= capture_and_report(&mut mic, 3) > 0;
    }
    assert!(any, "no enumerated microphone delivered audio");
}
