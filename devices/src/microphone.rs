use alsa::{Direction, PCM};
use anyhow::{Error, Ok, Result};
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
        callback: Box<dyn Fn(FramePacket) + Send + 'static>,
    ) -> Result<JoinHandle<()>, Error> {
        assert!(
            self.device_status() == &DeviceStatus::Available,
            "Camera is not in available status"
        );

        let device_info = self.device_info.clone();

        let (tx, rx) = flume::bounded::<DeviceCommand>(32);
        self.command_tx = Some(tx);

        let handle = spawn(move || {
            let pcm = PCM::new(&device_info.id, Direction::Capture, false).unwrap();
            let mut io = pcm.io_u8().unwrap();
            loop {
                match rx.try_recv() {
                    Err(err) => match err {
                        TryRecvError::Disconnected => break,
                        TryRecvError::Empty => continue,
                    },
                    c => match c.unwrap() {
                        DeviceCommand::Stop => {
                            break;
                        }
                        _ => {}
                    },
                }

                let mut buffer = Vec::<u8>::new();
                buffer.resize(config.buffer_size as usize, 0);
                io.read_exact(buffer.as_mut_slice()).unwrap();

                let fp = FramePacket::new(
                    FramePacketInformation {
                        device_info: device_info.clone(),
                        rx_timestamp: None,
                        tx_timestamp: None,
                        frame_shape: vec![config.buffer_size],
                    },
                    buffer.into_boxed_slice(),
                );

                callback(fp);
            }
            io.flush().unwrap();
        });
        Ok(handle)
    }

    fn stop(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

// pub struct CpalMicrophoneDevice {
//     pub device_info: DeviceInformation,
//     pub microphone: cpal::Device,
//     pub stream: Option<cpal::Stream>,
// }

// fn config_ranges_to_configs(
//     config_ranges: Vec<SupportedStreamConfigRange>,
// ) -> Result<Vec<MicrophoneConfig>, Error> {
//     let standard_channels: Vec<u16> = vec![1, 2];
//     let standard_sample_rates: Vec<u32> = vec![8000, 16000, 22050, 44100, 48000, 96000];
//     let standard_buffer_sizes: Vec<u32> = vec![128, 256, 512, 1024, 2048, 4096];

//     let mut out: Vec<MicrophoneConfig> = Vec::new();

//     for cr in config_ranges.iter() {
//         if !standard_channels.contains(&cr.channels()) {
//             continue;
//         }
//         if cr.sample_format() != SampleFormat::I16 {
//             continue;
//         }

//         for sample_rate in standard_sample_rates.iter() {
//             if sample_rate < &cr.min_sample_rate().0 && sample_rate > &cr.max_sample_rate().0 {
//                 continue;
//             }

//             for buffer_size in standard_buffer_sizes.iter() {
//                 let buffer_size_range = cr.buffer_size();

//                 match buffer_size_range {
//                     SupportedBufferSize::Range { min, max } => {
//                         if buffer_size >= min && buffer_size <= max {
//                             out.push(MicrophoneConfig {
//                                 sample_rate: *sample_rate,
//                                 channels: cr.channels(),
//                                 buffer_size: *buffer_size,
//                             });
//                         }
//                     }
//                     _ => {}
//                 }
//             }
//         }
//     }

//     Ok(out)
// }

// impl CpalMicrophoneDevice {
//     pub fn new(mic: CpalDevice, id: String) -> Self {
//         let device_info = DeviceInformation {
//             id: id,
//             name: mic.name().unwrap(),
//             device_type: DeviceType::Microphone,
//         };

//         CpalMicrophoneDevice {
//             device_info: device_info,
//             microphone: mic,
//             stream: None,
//         }
//     }

//     pub fn build_stream_configs(
//         &self,
//         mic_config: &MicrophoneConfig,
//     ) -> Result<StreamConfig, Error> {
//         let stream_config = StreamConfig {
//             channels: mic_config.channels,
//             sample_rate: cpal::SampleRate(mic_config.sample_rate),
//             buffer_size: cpal::BufferSize::Fixed(mic_config.buffer_size),
//         };
//         return Ok(stream_config);
//     }
// }

// impl Device for CpalMicrophoneDevice {
//     type Config = MicrophoneConfig;

//     fn id(&self) -> &str {
//         &self.device_info.id
//     }

//     fn name(&self) -> &str {
//         &self.device_info.name
//     }

//     fn device_type(&self) -> &DeviceType {
//         &self.device_info.device_type
//     }

//     fn get_configs(&self) -> Result<Vec<MicrophoneConfig>, Error> {
//         let conf_ranges = self
//             .microphone
//             .supported_input_configs()
//             .unwrap()
//             .collect::<Vec<SupportedStreamConfigRange>>();

//         Ok(config_ranges_to_configs(conf_ranges).unwrap())
//     }

//     fn open(
//         &mut self,
//         config: MicrophoneConfig,
//         callback: Box<dyn Fn(FramePacket) + Send + 'static>,
//         timeout: Option<Duration>,
//     ) -> Result<(), Error> {
//         assert!(
//             self.stream.is_none(),
//             "Microphone is already open (stream is not None)"
//         );

//         let stream_config = self.build_stream_configs(&config).unwrap();

//         println!(
//             "Config used for microphone - {:?} - {:?}",
//             self.device_info.name, stream_config
//         );

//         let di = self.device_info.clone();

//         let stream = self
//             .microphone
//             .build_input_stream_raw(
//                 &stream_config,
//                 SampleFormat::I16,
//                 move |data, _| {
//                     // process audio data
//                     let frame_shape: Vec<u32> = vec![
//                         config.sample_rate,
//                         config.channels as u32,
//                         config.buffer_size,
//                     ];

//                     let frame_packet = FramePacket::new(
//                         FramePacketInformation {
//                             device_info: di.clone(),
//                             rx_timestamp: None,
//                             tx_timestamp: None,
//                             frame_shape: frame_shape,
//                         },
//                         data.bytes().to_vec().into_boxed_slice(),
//                     );

//                     callback(frame_packet);
//                 },
//                 |err| {
//                     println!("Error from audio stream: {}", err);
//                 },
//                 timeout,
//             )
//             .unwrap();

//         stream.play().unwrap();

//         self.stream = Some(stream);

//         // TODO: listen to control signal here or persist stream

//         Ok(())
//     }

//     fn close(&mut self) -> Result<(), Error> {
//         if let Some(stream) = self.stream.take() {
//             stream.pause()?;
//         }
//         self.stream = None;
//         Ok(())
//     }
// }
