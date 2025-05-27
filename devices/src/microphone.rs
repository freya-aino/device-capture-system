use anyhow::{Error, Result};
use cpal::traits::HostTrait;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig, SupportedStreamConfigRange};
use std::time::Duration;

use shared::{DeviceInformation, DeviceType};

pub struct CpalMicrophoneDevice {
    pub device_info: DeviceInformation,
    pub microphone: cpal::Device,
    pub microphone_configs: Vec<SupportedStreamConfigRange>,
    pub stream: Option<cpal::Stream>,
}

impl CpalMicrophoneDevice {
    pub fn new(id: u16, cpal_host: &cpal::Host) -> Self {
        let mic = cpal_host
            .input_devices()
            .unwrap()
            .nth(id as usize)
            .ok_or_else(|| Error::msg("Device not found"))
            .unwrap();

        let conf = mic
            .supported_input_configs()
            .unwrap()
            .collect::<Vec<SupportedStreamConfigRange>>();

        let device_info = DeviceInformation {
            id: id,
            name: mic.name().unwrap(),
            device_type: DeviceType::Microphone,
        };

        CpalMicrophoneDevice {
            device_info: device_info,
            microphone: mic,
            microphone_configs: conf,
            stream: None,
        }
    }

    pub fn get_name(&self) -> String {
        self.device_info.name.clone()
    }

    pub fn print_configurations(&self) {
        for (i, conf) in self.microphone_configs.iter().enumerate() {
            println!("Microphone: {} - {:?}", i, conf);
        }
    }

    pub fn open(
        &mut self,
        timeout: Option<Duration>,
        sample_rate: Option<u32>,
        channels: Option<u16>,
        buffer_size: Option<u32>,
        sample_format: Option<SampleFormat>,
    ) -> Result<(), Error> {
        let supported_config = self.microphone.default_input_config().unwrap();

        let stream_config = StreamConfig {
            channels: channels.unwrap_or(supported_config.channels()),
            sample_rate: match sample_rate {
                Some(rate) => cpal::SampleRate(rate),
                None => supported_config.sample_rate(),
            },
            buffer_size: match buffer_size {
                Some(size) => cpal::BufferSize::Fixed(size),
                None => cpal::BufferSize::Default,
            },
        };

        let sample_format = sample_format.unwrap_or(supported_config.sample_format());

        let stream = self
            .microphone
            .build_input_stream_raw(
                &stream_config,
                sample_format,
                |data, info| {
                    // process audio data
                    let bytes = data.bytes();

                    println!("Received {} bytes of audio data", bytes.len());

                    // TODO: process audio data here
                },
                |err| {
                    println!("Error from audio stream: {}", err);
                },
                timeout,
            )
            .unwrap();

        stream.play().unwrap();

        self.stream = Some(stream);

        // TODO: listen to control signal here or persist stream

        Ok(())
    }

    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(stream) = self.stream.take() {
            stream.pause()?;
        }
        self.stream = None;
        Ok(())
    }
}
