use alsa::{
    Direction, PCM, ValueOr,
    device_name::HintIter,
    pcm::{Access, Format, HwParams},
};
use anyhow::{Error, Result, anyhow};
use flume::{Sender, TryRecvError};
use std::{
    io::{Read, Write},
    thread::{JoinHandle, spawn},
};

use shared::{
    Device, DeviceCommand, DeviceInformation, DeviceStatus, DeviceType, FramePacket,
    FramePacketInformation, MicrophoneConfig,
};

pub struct AlsaMicrophoneDevice {
    device_info: DeviceInformation,
    command_tx: Option<Sender<DeviceCommand>>,
}

impl AlsaMicrophoneDevice {
    pub fn new(id: String, name: String) -> Self {
        Self {
            device_info: DeviceInformation {
                id: id.to_string(),
                name: name.to_string(),
                device_type: DeviceType::Microphone,
                device_status: shared::DeviceStatus::Available,
            },
            command_tx: None,
        }
    }

    pub fn get_all_alsa_microphones() -> Result<Vec<AlsaMicrophoneDevice>, Error> {
        let hints = HintIter::new_str(None, "pcm")?
            .map(|hint| PcmHint {
                name: hint.name,
                desc: hint.desc,
                direction: hint.direction,
            })
            .collect::<Vec<_>>();

        Ok(select_capture_hints(hints)
            .into_iter()
            .map(|(id, name)| AlsaMicrophoneDevice::new(id, name))
            .collect())
    }
}

/// The three fields of an ALSA PCM hint that enumeration looks at. Mirrors `alsa::device_name::Hint`
/// so the selection rule can be tested without a sound card.
#[derive(Debug, Clone)]
pub struct PcmHint {
    pub name: Option<String>,
    pub desc: Option<String>,
    pub direction: Option<Direction>,
}

/// Picks the PCM hints worth offering as microphones, as `(id, display name)` pairs, in hint order.
///
/// Rules, in order:
/// 1. Playback-only hints and hints without a name are skipped.
/// 2. `null` and the bare `default`/`sysdefault` aliases are skipped: they duplicate another entry.
/// 3. Sound-server PCMs (`pipewire`, `pulse`, `jack`) are kept as they are. On a desktop the server owns the
///    sound card, so these are usually the only PCMs that can actually be opened for capture.
/// 4. Of the hints that name a card (`…:CARD=<card>…`), one per card is kept. The preferred plugin is the
///    one that converts formats for us: `sysdefault` > `plughw` > `front` > `hw` > anything else. The
///    earlier `DEV` substring filter removed `front:CARD=…,DEV=0`, the only hardware hint PipeWire hosts
///    expose, which left the list empty (`docs/hardware/2026-10-30-framework-12-webcam.md`).
pub fn select_capture_hints(hints: Vec<PcmHint>) -> Vec<(String, String)> {
    const SOUND_SERVERS: [&str; 3] = ["pipewire", "pulse", "jack"];
    const CARD_PLUGIN_PREFERENCE: [&str; 4] = ["sysdefault", "plughw", "front", "hw"];

    let display_name = |id: &str, desc: Option<String>| -> String {
        desc.map(|d| d.lines().next().unwrap_or("").trim().to_string())
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| id.to_string())
    };
    let plugin_rank = |base: &str| {
        CARD_PLUGIN_PREFERENCE
            .iter()
            .position(|p| *p == base)
            .unwrap_or(CARD_PLUGIN_PREFERENCE.len())
    };

    let mut servers: Vec<(String, String)> = Vec::new();
    // card -> (rank, hint index, id, name); lower rank wins, earlier hint breaks ties
    let mut per_card: Vec<(String, usize, usize, String, String)> = Vec::new();

    for (index, hint) in hints.into_iter().enumerate() {
        if hint.direction == Some(Direction::Playback) {
            continue;
        }
        let Some(id) = hint.name else {
            continue;
        };
        let base = id.split(':').next().unwrap_or(&id).to_string();
        let card = id
            .split(':')
            .nth(1)
            .and_then(|args| args.split(',').find_map(|kv| kv.strip_prefix("CARD=")))
            .map(str::to_string);

        match (base.as_str(), card) {
            ("null", _) => {}
            ("default" | "sysdefault", None) => {}
            (b, _) if SOUND_SERVERS.contains(&b) => {
                let name = display_name(&id, hint.desc);
                servers.push((id, name));
            }
            (_, None) => {}
            (b, Some(card)) => {
                let rank = plugin_rank(b);
                let name = display_name(&id, hint.desc);
                match per_card.iter_mut().find(|entry| entry.0 == card) {
                    Some(entry) if rank < entry.1 => *entry = (card, rank, index, id, name),
                    Some(_) => {}
                    None => per_card.push((card, rank, index, id, name)),
                }
            }
        }
    }

    per_card.sort_by_key(|entry| entry.2);
    per_card
        .into_iter()
        .map(|(_, _, _, id, name)| (id, name))
        .chain(servers)
        .collect()
}

impl Device for AlsaMicrophoneDevice {
    type Config = MicrophoneConfig;

    fn id(&self) -> &str {
        &self.device_info.id
    }
    fn name(&self) -> &str {
        &self.device_info.name
    }
    fn device_type(&self) -> &DeviceType {
        &self.device_info.device_type
    }
    fn device_status(&self) -> &shared::DeviceStatus {
        &self.device_info.device_status
    }

    fn get_configs(&self) -> std::result::Result<Vec<Self::Config>, Error> {
        Ok(vec![])
    }

    fn start(
        &mut self,
        config: MicrophoneConfig,
        callback: Box<dyn Fn(FramePacket) -> Result<(), Error> + Send + 'static>,
    ) -> Result<JoinHandle<Result<(), Error>>, Error> {
        assert!(
            self.device_status() != &DeviceStatus::Running,
            "Microphone is already running"
        );

        let device_info = self.device_info.clone();

        let (tx, rx) = flume::bounded::<DeviceCommand>(32);
        self.command_tx = Some(tx);

        let handle = spawn(move || -> Result<(), Error> {
            let pcm = PCM::new(&device_info.id, Direction::Capture, false)?;

            // set parameters
            let params = HwParams::any(&pcm)?;
            params.set_channels(config.channels as u32)?;
            params.set_rate(config.sample_rate, ValueOr::Nearest)?;
            params.set_format(Format::S16LE)?;
            params.set_access(Access::RWInterleaved)?;

            tracing::debug!("Desired microphone configurations {:?}", params);

            // start pcm
            pcm.hw_params(&params)?;
            pcm.start()?;
            let mut io = pcm.io_i16()?;

            tracing::debug!("Config used by Microphone: {:?}", pcm.get_params());
            tracing::info!("Microphone started");

            loop {
                match rx.try_recv() {
                    Err(err) => match err {
                        TryRecvError::Disconnected => break,
                        TryRecvError::Empty => {}
                    },
                    Ok(DeviceCommand::Stop) => break,
                    Ok(_) => {}
                }

                // capture buffer (i16 little-endian --> u8 byte level representation)
                let mut buffer = Vec::<u8>::new();
                buffer.resize(
                    config.buffer_size as usize * config.channels as usize * 2,
                    0,
                );
                io.read_exact(&mut buffer)?;

                let fp = FramePacket::new(
                    FramePacketInformation {
                        device_info: device_info.clone(),
                        rx_timestamp: None,
                        tx_timestamp: None,
                        frame_shape: vec![config.buffer_size, config.channels as u32],
                        pixel_format: None,
                    },
                    buffer.into_boxed_slice(),
                );

                callback(fp)?;
            }
            io.flush()?;
            Ok(())
        });
        self.device_info.device_status = DeviceStatus::Running;
        Ok(handle)
    }

    fn stop(&mut self) -> Result<(), Error> {
        match self.command_tx.take() {
            Some(tx) => {
                // A failed send means the capture thread has already exited (it dropped the
                // receiver, e.g. after a read error). The device is stopped either way, so the
                // status must say so or the node could never restart it.
                if tx.send(DeviceCommand::Stop).is_err() {
                    tracing::warn!("capture thread had already exited before stop()");
                }
                self.device_info.device_status = DeviceStatus::Stopped;
                Ok(())
            }
            None => Err(anyhow!(
                "Tried stopping while command channel not initialized"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hint(name: &str, desc: &str, direction: Option<Direction>) -> PcmHint {
        PcmHint {
            name: Some(name.to_string()),
            desc: Some(desc.to_string()),
            direction,
        }
    }

    /// `arecord -L` on the Framework Laptop 12 with PipeWire (docs/hardware/2026-10-30-framework-12-webcam.md).
    fn framework_pipewire_hints() -> Vec<PcmHint> {
        vec![
            hint(
                "null",
                "Discard all samples (playback) or generate zero samples (capture)",
                None,
            ),
            hint("pipewire", "PipeWire Sound Server", None),
            hint(
                "default",
                "Default ALSA Output (currently PipeWire Media Server)",
                None,
            ),
            hint(
                "sysdefault:CARD=PCH",
                "HDA Intel PCH, ALC285 Analog\nDefault Audio Device",
                None,
            ),
            hint(
                "front:CARD=PCH,DEV=0",
                "HDA Intel PCH, ALC285 Analog\nFront output / input",
                None,
            ),
            hint(
                "hdmi:CARD=PCH,DEV=0",
                "HDA Intel PCH, HDMI 0\nHDMI Audio Output",
                Some(Direction::Playback),
            ),
        ]
    }

    #[test]
    fn pipewire_host_keeps_one_hardware_pcm_per_card_and_the_sound_server() {
        let picked = select_capture_hints(framework_pipewire_hints());
        assert_eq!(
            picked,
            vec![
                (
                    "sysdefault:CARD=PCH".to_string(),
                    "HDA Intel PCH, ALC285 Analog".to_string()
                ),
                ("pipewire".to_string(), "PipeWire Sound Server".to_string()),
            ]
        );
    }

    #[test]
    fn front_hint_survives_when_it_is_the_only_hardware_hint() {
        let hints = vec![
            hint("default", "Default ALSA Output", None),
            hint(
                "front:CARD=PCH,DEV=0",
                "HDA Intel PCH, ALC285 Analog\nFront output / input",
                None,
            ),
        ];
        let picked = select_capture_hints(hints);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].0, "front:CARD=PCH,DEV=0");
    }

    #[test]
    fn classic_alsa_host_prefers_the_converting_plugin_and_skips_playback_only() {
        let hints = vec![
            hint(
                "hw:CARD=USB,DEV=0",
                "USB Audio\nDirect hardware device without any conversions",
                None,
            ),
            hint(
                "plughw:CARD=USB,DEV=0",
                "USB Audio\nHardware device with all software conversions",
                None,
            ),
            hint(
                "dmix:CARD=USB,DEV=0",
                "USB Audio\nDirect sample mixing device",
                Some(Direction::Playback),
            ),
            hint(
                "hw:CARD=Mic2,DEV=0",
                "Second Mic\nDirect hardware device",
                None,
            ),
            hint("sysdefault", "Default Audio Device", None),
        ];
        let picked = select_capture_hints(hints);
        assert_eq!(
            picked.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            vec!["plughw:CARD=USB,DEV=0", "hw:CARD=Mic2,DEV=0"]
        );
        assert_eq!(picked[0].1, "USB Audio");
    }

    #[test]
    fn hints_without_a_name_or_only_null_yield_nothing() {
        let hints = vec![
            PcmHint {
                name: None,
                desc: Some("x".into()),
                direction: None,
            },
            hint("null", "Discard all samples", None),
        ];
        assert!(select_capture_hints(hints).is_empty());
    }
}
