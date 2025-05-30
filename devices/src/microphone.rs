use anyhow::{Error, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device as CpalDevice, SampleFormat, SupportedBufferSize};
use cpal::{StreamConfig, SupportedStreamConfigRange};
use std::time::Duration;

use shared::{Device, DeviceInformation, DeviceType, MicrophoneConfig};

pub struct CpalMicrophoneDevice {
    pub device_info: DeviceInformation,
    pub microphone: cpal::Device,
    pub stream: Option<cpal::Stream>,
}

fn config_ranges_to_configs(
    config_ranges: Vec<SupportedStreamConfigRange>,
) -> Result<Vec<MicrophoneConfig>, Error> {
    let standard_channels: Vec<u16> = vec![1, 2];
    let standard_sample_rates: Vec<u32> = vec![8000, 16000, 22050, 44100, 48000, 96000];
    let standard_buffer_sizes: Vec<u32> = vec![128, 256, 512, 1024, 2048, 4096];

    let mut out: Vec<MicrophoneConfig> = Vec::new();

    for cr in config_ranges.iter() {
        if !standard_channels.contains(&cr.channels()) {
            continue;
        }
        if cr.sample_format() != SampleFormat::I16 {
            continue;
        }

        for sample_rate in standard_sample_rates.iter() {
            if sample_rate < &cr.min_sample_rate().0 && sample_rate > &cr.max_sample_rate().0 {
                continue;
            }

            for buffer_size in standard_buffer_sizes.iter() {
                let buffer_size_range = cr.buffer_size();

                match buffer_size_range {
                    SupportedBufferSize::Range { min, max } => {
                        if buffer_size >= min && buffer_size <= max {
                            out.push(MicrophoneConfig {
                                sample_rate: *sample_rate,
                                channels: cr.channels(),
                                buffer_size: *buffer_size,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(out)
}

impl CpalMicrophoneDevice {
    pub fn new(mic: CpalDevice, id: u16) -> Self {
        let device_info = DeviceInformation {
            id: id,
            name: mic.name().unwrap(),
            device_type: DeviceType::Microphone,
        };

        CpalMicrophoneDevice {
            device_info: device_info,
            microphone: mic,
            stream: None,
        }
    }

    pub fn build_stream_configs(
        &self,
        mic_config: MicrophoneConfig,
    ) -> Result<StreamConfig, Error> {
        let stream_config = StreamConfig {
            channels: mic_config.channels,
            sample_rate: cpal::SampleRate(mic_config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(mic_config.buffer_size),
        };
        return Ok(stream_config);
    }
}

impl Device for CpalMicrophoneDevice {
    type Config = MicrophoneConfig;

    fn id(&self) -> u16 {
        self.device_info.id
    }

    fn name(&self) -> &str {
        &self.device_info.name
    }

    fn device_type(&self) -> &DeviceType {
        &self.device_info.device_type
    }

    fn get_configs(&self) -> Result<Vec<MicrophoneConfig>, Error> {
        let conf_ranges = self
            .microphone
            .supported_input_configs()
            .unwrap()
            .collect::<Vec<SupportedStreamConfigRange>>();

        Ok(config_ranges_to_configs(conf_ranges).unwrap())
    }

    fn open(&mut self, config: MicrophoneConfig, timeout: Option<Duration>) -> Result<(), Error> {
        let stream_config = self.build_stream_configs(config).unwrap();

        let stream = self
            .microphone
            .build_input_stream_raw(
                &stream_config,
                SampleFormat::I16,
                |data, _| {
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

    fn close(&mut self) -> Result<(), Error> {
        if let Some(stream) = self.stream.take() {
            stream.pause()?;
        }
        self.stream = None;
        Ok(())
    }
}
