use std::net::Ipv4Addr;
use std::time::{Duration, UNIX_EPOCH};

use access_client::AccessClient;
use anyhow::Error;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use shared::{DeviceType, FramePacket};

fn to_py_err(e: Error) -> PyErr {
    PyRuntimeError::new_err(format!("{e:#}"))
}

fn timestamp_secs(t: Option<std::time::SystemTime>) -> Option<f64> {
    t.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
}

/// One frame received through the hub. Timestamps are seconds since the Unix epoch.
#[pyclass(module = "device_system")]
pub struct Frame {
    device_id: String,
    device_name: String,
    device_type: String,
    frame_shape: Vec<u32>,
    rx_timestamp: Option<f64>,
    tx_timestamp: Option<f64>,
    data: Vec<u8>,
}

impl From<FramePacket> for Frame {
    fn from(packet: FramePacket) -> Self {
        let info = packet.frame_info;
        let device_type = match info.device_info.device_type {
            DeviceType::Camera => "camera",
            DeviceType::Microphone => "microphone",
        };
        Frame {
            device_id: info.device_info.id,
            device_name: info.device_info.name,
            device_type: device_type.to_string(),
            frame_shape: info.frame_shape,
            rx_timestamp: timestamp_secs(info.rx_timestamp),
            tx_timestamp: timestamp_secs(info.tx_timestamp),
            data: packet.data.into_vec(),
        }
    }
}

#[pymethods]
impl Frame {
    #[getter]
    fn device_id(&self) -> &str {
        &self.device_id
    }

    #[getter]
    fn device_name(&self) -> &str {
        &self.device_name
    }

    #[getter]
    fn device_type(&self) -> &str {
        &self.device_type
    }

    #[getter]
    fn frame_shape(&self) -> Vec<u32> {
        self.frame_shape.clone()
    }

    #[getter]
    fn rx_timestamp(&self) -> Option<f64> {
        self.rx_timestamp
    }

    #[getter]
    fn tx_timestamp(&self) -> Option<f64> {
        self.tx_timestamp
    }

    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    fn __repr__(&self) -> String {
        format!(
            "Frame(device_id={:?}, device_type={:?}, frame_shape={:?}, bytes={})",
            self.device_id,
            self.device_type,
            self.frame_shape,
            self.data.len()
        )
    }
}

/// Consumer-side handle onto a `control-hub`'s XPUB endpoint.
///
/// `ds = DeviceSystem(hub_ip, hub_port)` followed by `for frame in ds.get_frame():`
/// blocks for each frame. Ctrl+C interrupts the loop. Must be used from the thread that created it.
#[pyclass(module = "device_system", unsendable)]
pub struct DeviceSystem {
    client: AccessClient,
}

#[pymethods]
impl DeviceSystem {
    #[new]
    fn new(hub_ip: &str, hub_port: u16) -> PyResult<Self> {
        let ip: Ipv4Addr = hub_ip
            .parse()
            .map_err(|e| PyValueError::new_err(format!("invalid hub_ip {hub_ip:?}: {e}")))?;
        let mut client = AccessClient::new(ip, hub_port, 10, 1024);
        client.connect().map_err(to_py_err)?;
        Ok(DeviceSystem { client })
    }

    /// Returns the next frame, or `None` if none arrives within `timeout_ms`.
    #[pyo3(signature = (timeout_ms = 1000))]
    fn next_frame(&mut self, py: Python<'_>, timeout_ms: u64) -> PyResult<Option<Frame>> {
        let client = &mut self.client;
        let packet = py
            .detach(|| client.next_frame(Duration::from_millis(timeout_ms)))
            .map_err(to_py_err)?;
        Ok(packet.map(Frame::from))
    }

    /// Iterator over frames, for use in `for frame in ds.get_frame():`.
    fn get_frame(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Frame> {
        loop {
            if let Some(frame) = self.next_frame(py, 100)? {
                return Ok(frame);
            }
            py.check_signals()?;
        }
    }

    fn close(&mut self) -> PyResult<()> {
        self.client.disconnect().map_err(to_py_err)
    }
}

#[pymodule]
fn device_system(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DeviceSystem>()?;
    m.add_class::<Frame>()?;
    Ok(())
}
