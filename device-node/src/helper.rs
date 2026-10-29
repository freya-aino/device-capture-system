use std::{thread, time::Duration};

use anyhow::{Error, Result, anyhow};
use devices::{AlsaMicrophoneDevice, V4lCameraDevice};
use shared::{
    CameraConfig, CameraSelectionPolicy, Device, FramePacket, MicrophoneConfig,
    select_camera_config, select_camera_config_by_policy,
};

pub static DEFAULT_MICROPHONE_CONFIG: MicrophoneConfig = MicrophoneConfig {
    sample_rate: 16000,
    channels: 1,
    buffer_size: 512,
};

/// Requested camera config. The node does not stream this as-is: it picks the advertised config
/// nearest to it with `select_camera_config` (see `CameraSelection::Nearest`). Only the ignored
/// hardware helpers and tests use it directly.
pub static DEFAULT_CAMERA_CONFIG: CameraConfig = CameraConfig {
    width: 1920,
    height: 1080,
    fps: (1, 30),
    fourcc: ['Y', 'U', 'Y', 'V'],
};

/// How `device-node run camera` picks one of the configs the camera advertises.
#[derive(Debug, Clone, PartialEq)]
pub enum CameraSelection {
    /// Closest advertised resolution, then closest frame rate, uncompressed formats winning ties.
    Nearest { width: u32, height: u32, fps: f64 },
    /// One of the ranking policies in `shared::CameraSelectionPolicy`.
    Policy(CameraSelectionPolicy),
}

impl Default for CameraSelection {
    fn default() -> Self {
        CameraSelection::Nearest {
            width: DEFAULT_CAMERA_CONFIG.width,
            height: DEFAULT_CAMERA_CONFIG.height,
            fps: DEFAULT_CAMERA_CONFIG.frames_per_second(),
        }
    }
}

impl CameraSelection {
    /// Reads `DEVICE_NODE_CAMERA_POLICY` (`nearest`, `best-fps`, `best-resolution`, `best-average`;
    /// default `nearest`) and, for `nearest`, `DEVICE_NODE_CAMERA_WIDTH`, `_HEIGHT` and `_FPS`
    /// (defaults 1920, 1080, 30).
    pub fn from_env() -> Result<Self, Error> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        Self::from_values(
            var("DEVICE_NODE_CAMERA_POLICY").as_deref(),
            var("DEVICE_NODE_CAMERA_WIDTH").as_deref(),
            var("DEVICE_NODE_CAMERA_HEIGHT").as_deref(),
            var("DEVICE_NODE_CAMERA_FPS").as_deref(),
        )
    }

    pub fn from_values(
        policy: Option<&str>,
        width: Option<&str>,
        height: Option<&str>,
        fps: Option<&str>,
    ) -> Result<Self, Error> {
        match policy.map(|p| p.trim().to_ascii_lowercase()).as_deref() {
            None | Some("nearest") => {
                let CameraSelection::Nearest {
                    width: dw,
                    height: dh,
                    fps: dfps,
                } = CameraSelection::default()
                else {
                    unreachable!("default selection is Nearest")
                };
                let parse_u32 =
                    |name: &str, value: Option<&str>, default: u32| -> Result<u32, Error> {
                        value.map_or(Ok(default), |v| {
                            v.trim().parse::<u32>().map_err(|e| {
                                anyhow!("{name} must be a positive integer, got {v:?}: {e}")
                            })
                        })
                    };
                let fps = fps.map_or(Ok(dfps), |v| {
                    v.trim().parse::<f64>().map_err(|e| {
                        anyhow!("DEVICE_NODE_CAMERA_FPS must be a number, got {v:?}: {e}")
                    })
                })?;
                Ok(CameraSelection::Nearest {
                    width: parse_u32("DEVICE_NODE_CAMERA_WIDTH", width, dw)?,
                    height: parse_u32("DEVICE_NODE_CAMERA_HEIGHT", height, dh)?,
                    fps,
                })
            }
            Some(other) => other
                .parse::<CameraSelectionPolicy>()
                .map(CameraSelection::Policy)
                .map_err(|e| anyhow!("DEVICE_NODE_CAMERA_POLICY: {e}")),
        }
    }

    /// Applies the selection to the advertised list. `None` when the camera advertises nothing.
    pub fn choose(&self, configs: &[CameraConfig]) -> Option<CameraConfig> {
        match *self {
            CameraSelection::Nearest { width, height, fps } => {
                select_camera_config(configs, width, height, fps)
            }
            CameraSelection::Policy(policy) => select_camera_config_by_policy(configs, policy),
        }
    }
}

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
    println!("picked mic: {:?}", mic.id());

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

    fn config(width: u32, height: u32, fps: (u32, u32), fourcc: &str) -> CameraConfig {
        let mut chars = fourcc.chars();
        CameraConfig {
            width,
            height,
            fps,
            fourcc: std::array::from_fn(|_| chars.next().unwrap()),
        }
    }

    /// The list the Framework Laptop 12 webcam advertises (docs/hardware/2026-10-30-framework-12-webcam.md).
    fn framework_webcam() -> Vec<CameraConfig> {
        vec![
            config(1920, 1080, (1, 60), "MJPG"),
            config(1920, 1080, (1, 30), "MJPG"),
            config(1280, 720, (1, 60), "MJPG"),
            config(1280, 720, (1, 30), "MJPG"),
            config(640, 480, (1, 60), "MJPG"),
            config(640, 480, (1, 30), "MJPG"),
            config(640, 360, (1, 60), "MJPG"),
            config(640, 360, (1, 30), "MJPG"),
            config(640, 480, (1, 30), "YUYV"),
            config(640, 360, (1, 30), "YUYV"),
        ]
    }

    #[test]
    fn camera_selection_default_is_nearest_to_the_default_config() {
        assert_eq!(
            CameraSelection::default(),
            CameraSelection::Nearest {
                width: 1920,
                height: 1080,
                fps: 30.0
            }
        );
        let picked = CameraSelection::default()
            .choose(&framework_webcam())
            .unwrap();
        assert_eq!(
            (picked.width, picked.height, picked.fps),
            (1920, 1080, (1, 30))
        );
        assert_eq!(picked.fourcc_string(), "MJPG");
    }

    #[test]
    fn camera_selection_nearest_reads_overrides() {
        let selection =
            CameraSelection::from_values(Some("nearest"), Some("640"), Some("480"), Some("30"))
                .unwrap();
        assert_eq!(
            selection,
            CameraSelection::Nearest {
                width: 640,
                height: 480,
                fps: 30.0
            }
        );
        let picked = selection.choose(&framework_webcam()).unwrap();
        assert_eq!((picked.width, picked.height), (640, 480));
        assert_eq!(
            picked.fourcc_string(),
            "YUYV",
            "uncompressed wins the exact tie"
        );
    }

    #[test]
    fn camera_selection_parses_policies_and_rejects_bad_values() {
        assert_eq!(
            CameraSelection::from_values(Some("best-fps"), None, None, None).unwrap(),
            CameraSelection::Policy(CameraSelectionPolicy::BestFps)
        );
        let picked = CameraSelection::Policy(CameraSelectionPolicy::BestFps)
            .choose(&framework_webcam())
            .unwrap();
        assert_eq!(
            (picked.width, picked.height, picked.fps),
            (1920, 1080, (1, 60))
        );

        assert!(CameraSelection::from_values(Some("fastest"), None, None, None).is_err());
        assert!(CameraSelection::from_values(None, Some("wide"), None, None).is_err());
        assert!(CameraSelection::from_values(None, None, None, Some("many")).is_err());
    }

    #[test]
    fn camera_selection_returns_none_for_empty_list() {
        assert!(CameraSelection::default().choose(&[]).is_none());
    }

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
