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
        let hints = HintIter::new_str(None, "pcm")?;

        let mut out_devices = Vec::<AlsaMicrophoneDevice>::new();
        for hint in hints {
            if hint.direction == Some(Direction::Playback) {
                continue;
            }

            let Some(id) = hint.name else {
                continue;
            };
            let name = hint.desc.unwrap_or_else(|| id.clone());

            let base = id.split(':').next().unwrap_or(&id);
            if matches!(base, "default" | "null" | "pipewire" | "sysdefault") {
                continue;
            }

            if id.contains("DEV") {
                continue;
            }

            out_devices.push(AlsaMicrophoneDevice::new(id, name));
        }

        Ok(out_devices)
    }
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
                buffer.resize(config.buffer_size as usize * config.channels as usize * 2, 0);
                io.read_exact(&mut buffer)?;

                let fp = FramePacket::new(
                    FramePacketInformation {
                        device_info: device_info.clone(),
                        rx_timestamp: None,
                        tx_timestamp: None,
                        frame_shape: vec![config.buffer_size, config.channels as u32],
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
                tx.send(DeviceCommand::Stop)
                    .map_err(|_| anyhow!("capture thread already stopped"))?;
                self.device_info.device_status = DeviceStatus::Stopped;
                Ok(())
            }
            None => Err(anyhow!(
                "Tried stopping while command channel not initialized"
            )),
        }
    }
}
