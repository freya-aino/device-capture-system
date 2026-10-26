use std::{thread, time::Duration};

use anyhow::{Error, Result, anyhow};
use devices::{AlsaMicrophoneDevice, V4lCameraDevice};
use shared::{CameraConfig, Device, FramePacket, MicrophoneConfig};

pub static DEFAULT_MICROPHONE_CONFIG: MicrophoneConfig = MicrophoneConfig {
    sample_rate: 16000,
    channels: 1,
    buffer_size: 512,
};

pub static DEFAULT_CAMERA_CONFIG: CameraConfig = CameraConfig {
    width: 1920,
    height: 1080,
    fps: (1, 30),
    fourcc: ['Y', 'U', 'Y', 'V'],
};

// Calculates RMS (root mean square) as a way to determine volume
pub fn volume_from_audio_frame(buf: &[i16]) -> f64 {
    let mut sum = 0f64;
    for &x in buf {
        let sq = (x as f64) * (x as f64);
        sum += sq;
    }
    let mean = sum / (buf.len() as f64);
    mean.sqrt() / (i16::MAX as f64)
}

pub fn get_all_devices() -> Result<(Vec<V4lCameraDevice>, Vec<AlsaMicrophoneDevice>), Error> {
    Ok((
        V4lCameraDevice::get_all_v4l_devices()?,
        AlsaMicrophoneDevice::get_all_alsa_microphones()?,
    ))
}

pub fn print_all_devices(
    all_cameras: &Vec<V4lCameraDevice>,
    all_microphones: &Vec<AlsaMicrophoneDevice>,
) {
    println!("num cameras: {:?}", all_cameras.len());
    for cam in all_cameras.iter() {
        println!("Camera: {:?} -- {}", cam.id(), cam.name());
    }

    println!("num microphones: {:?}", all_microphones.len());
    for mic in all_microphones.iter() {
        println!("Microphone: {:?} -- {}", mic.id(), mic.name());
    }
}

pub fn test_alsa_mic(mic: &mut AlsaMicrophoneDevice, seconds: u64) -> Result<(), Error> {
    println!("piced mic: {:?}", mic.id());

    let handle = mic.start(
        DEFAULT_MICROPHONE_CONFIG.clone(),
        Box::new(|fp: FramePacket| -> Result<(), Error> {
            println!("data len: {:?}", fp.data.len());

            let i16le = FramePacket::decode_to_i16_le(&fp.data)?;
            let vol = volume_from_audio_frame(&i16le);

            let volume_meter = "#".repeat((vol * 100.0) as usize);
            println!("vol: {}", volume_meter);
            Ok(())
        }),
    )?;

    thread::sleep(Duration::from_secs(seconds));

    mic.stop()?;

    let join_ = handle
        .join()
        .map_err(|e| anyhow!("Thread paniced when joining: {:?}", e));

    join_??;
    Ok(())
}

pub fn test_v4l_cameras(cam: &mut V4lCameraDevice, seconds: u64) -> Result<(), Error> {
    let handle = cam.start(
        DEFAULT_CAMERA_CONFIG.clone(),
        Box::new(|a| -> Result<(), Error> {
            println!("data len: {:?}", a.data.len());
            Ok(())
        }),
    )?;

    thread::sleep(Duration::from_secs(seconds));

    cam.stop()?;

    let join_ = handle
        .join()
        .map_err(|e| anyhow!("Thread paniced when joining: {:?}", e));

    join_??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_from_audio_frame_silence_is_zero() {
        let silence = vec![0i16; 10];
        assert_eq!(volume_from_audio_frame(&silence), 0.0);
    }

    #[test]
    fn volume_from_audio_frame_max_amplitude_is_one() {
        let loudest = vec![i16::MAX; 10];
        let vol = volume_from_audio_frame(&loudest);
        assert!((vol - 1.0).abs() < 1e-9);
    }

    #[test]
    fn volume_from_audio_frame_is_amplitude_independent_of_sign() {
        let positive = vec![1000i16; 10];
        let negative = vec![-1000i16; 10];
        assert_eq!(
            volume_from_audio_frame(&positive),
            volume_from_audio_frame(&negative)
        );
    }
}
