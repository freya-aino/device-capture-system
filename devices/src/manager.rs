use anyhow::Error;
use cpal::{Host, traits::HostTrait};
use glob::glob;
use shared::{Device, DeviceStatus};
use std::ffi::OsString;

use crate::{CpalMicrophoneDevice, V4lCameraDevice};

pub struct DeviceManager<D: Device> {
    pub device: D,
    pub device_status: DeviceStatus,
    pub ip: Option<String>,
    pub port: Option<u16>,
}

impl DeviceManager<CpalMicrophoneDevice> {
    pub fn new(
        device: CpalMicrophoneDevice,
        device_status: DeviceStatus,
        ip: Option<String>,
        port: Option<u16>,
    ) -> Self {
        Self {
            device: device,
            device_status: device_status,
            // rx: rx,
            ip: ip,
            port: port,
        }
    }

    pub fn get_all_devices(cpal_host: &Host) -> Result<Vec<Self>, Error> {
        // let host = cpal::default_host();
        let cpal_devices = cpal_host.input_devices()?;

        let mut devices = Vec::<CpalMicrophoneDevice>::new();
        for (i, dev) in cpal_devices.enumerate() {
            devices.push(CpalMicrophoneDevice::new(dev, i as u16));
        }

        let mut out = Vec::<Self>::new();
        for device in devices {
            out.push(Self::new(device, DeviceStatus::Available, None, None));
        }

        Ok(out)
    }
}

impl DeviceManager<V4lCameraDevice> {
    pub fn new(
        device: V4lCameraDevice,
        // rx: Receiver<DeviceCommand>,
        device_status: DeviceStatus,
        ip: Option<String>,
        port: Option<u16>,
    ) -> Self {
        Self {
            device: device,
            device_status: device_status,
            // rx: rx,
            ip: ip,
            port: port,
        }
    }

    pub fn get_all_devices() -> Result<Vec<Self>, Error> {
        let device_paths = glob("/dev/video*")?
            .filter_map(Result::ok)
            .map(|path| path.into_os_string())
            .collect::<Vec<OsString>>();

        let mut devices = Vec::<V4lCameraDevice>::new();
        for (i, dp) in device_paths.iter().enumerate() {
            devices.push(match dp.clone().into_string() {
                Ok(str_path) => V4lCameraDevice::new(&str_path, i as u16),
                Err(os_path) => V4lCameraDevice::new(&os_path.to_string_lossy(), i as u16),
            })
        }

        let mut out = Vec::<Self>::new();
        for device in devices {
            out.push(Self::new(device, DeviceStatus::Available, None, None));
        }

        Ok(out)
    }
}
