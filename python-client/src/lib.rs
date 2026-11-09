use std::net::Ipv4Addr;
use std::time::{Duration, UNIX_EPOCH};

use access_client::AccessClient;
use anyhow::Error;
use networking::{FrameGroup, GroupReceiver};
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
#[derive(Clone)]
pub struct Frame {
    device_id: String,
    device_name: String,
    device_type: String,
    frame_shape: Vec<u32>,
    pixel_format: Option<String>,
    sample_rate: Option<u32>,
    capture_timestamp: Option<f64>,
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
            pixel_format: info.pixel_format,
            sample_rate: info.sample_rate,
            capture_timestamp: timestamp_secs(info.capture_timestamp),
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

    /// FourCC of a camera frame (`"YUYV"`, `"MJPG"`, ...); `None` for microphone frames.
    #[getter]
    fn pixel_format(&self) -> Option<&str> {
        self.pixel_format.as_deref()
    }

    /// Sample rate in Hz of a microphone frame, as granted by the sound card; `None` for camera frames.
    #[getter]
    fn sample_rate(&self) -> Option<u32> {
        self.sample_rate
    }

    /// When the device captured the frame (seconds since the epoch), if the producer reported it.
    /// Use this, not `tx_timestamp`, to align frames from different devices.
    #[getter]
    fn capture_timestamp(&self) -> Option<f64> {
        self.capture_timestamp
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

    /// Pixels of a camera frame as `(width, height, channels, bytes)`, rows top to bottom:
    /// `RGB3` and `GREY` as they are, `YUYV` converted to RGB, `MJPG` decoded. Uses the same
    /// decoder as the hub's pre-processor. Raises `ValueError` for microphone frames and for
    /// payloads that do not decode.
    fn image<'py>(&self, py: Python<'py>) -> PyResult<(u32, u32, u32, Bound<'py, PyBytes>)> {
        if self.device_type != "camera" {
            return Err(PyValueError::new_err(format!(
                "image() needs a camera frame, this is a {} frame",
                self.device_type
            )));
        }
        let format = self
            .pixel_format
            .as_deref()
            .ok_or_else(|| PyValueError::new_err("camera frame has no pixel_format"))?;
        let image = preprocess::decode_payload(&self.data, format, &self.frame_shape)
            .map_err(|e| PyValueError::new_err(format!("{e:#}")))?;
        Ok((
            image.width,
            image.height,
            image.format.channels() as u32,
            PyBytes::new(py, &image.data),
        ))
    }

    fn __repr__(&self) -> String {
        format!(
            "Frame(device_id={:?}, device_type={:?}, frame_shape={:?}, pixel_format={:?}, bytes={})",
            self.device_id,
            self.device_type,
            self.frame_shape,
            self.pixel_format,
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

/// One aligned set of frames from the hub's sync port: one `Frame` per device, and the spread
/// of their capture times in seconds.
#[pyclass(module = "device_system")]
pub struct Group {
    frames: Vec<Frame>,
    skew_s: f64,
}

impl From<FrameGroup> for Group {
    fn from(group: FrameGroup) -> Self {
        Group {
            frames: group.frames.into_iter().map(Frame::from).collect(),
            skew_s: group.skew.as_secs_f64(),
        }
    }
}

#[pymethods]
impl Group {
    #[getter]
    fn frames(&self) -> Vec<Frame> {
        self.frames.clone()
    }

    #[getter]
    fn skew_s(&self) -> f64 {
        self.skew_s
    }

    fn __len__(&self) -> usize {
        self.frames.len()
    }

    fn __repr__(&self) -> String {
        let ids: Vec<&str> = self.frames.iter().map(|f| f.device_id.as_str()).collect();
        format!("Group(devices={ids:?}, skew_s={:.6})", self.skew_s)
    }
}

/// Consumer-side handle onto a `control-hub`'s sync port (`HUB_SYNC_PORT`), yielding aligned
/// groups. `for group in AlignedSystem(hub_ip, sync_port):` blocks for each group.
#[pyclass(module = "device_system", unsendable)]
pub struct AlignedSystem {
    receiver: GroupReceiver,
    _context: zmq::Context,
}

#[pymethods]
impl AlignedSystem {
    #[new]
    fn new(hub_ip: &str, sync_port: u16) -> PyResult<Self> {
        let ip: Ipv4Addr = hub_ip
            .parse()
            .map_err(|e| PyValueError::new_err(format!("invalid hub_ip {hub_ip:?}: {e}")))?;
        let context = zmq::Context::new();
        let mut receiver = GroupReceiver::new_endpoint(format!("tcp://{ip}:{sync_port}"), 10, 1024);
        receiver.start(&context).map_err(to_py_err)?;
        Ok(AlignedSystem {
            receiver,
            _context: context,
        })
    }

    /// Returns the next aligned group, or `None` if none arrives within `timeout_ms`.
    #[pyo3(signature = (timeout_ms = 1000))]
    fn next_group(&mut self, py: Python<'_>, timeout_ms: u64) -> PyResult<Option<Group>> {
        let receiver = &mut self.receiver;
        let group = py
            .detach(|| -> Result<Option<FrameGroup>, Error> {
                let start = std::time::Instant::now();
                let timeout = Duration::from_millis(timeout_ms);
                loop {
                    if let Some(group) = receiver.receive(zmq::DONTWAIT)? {
                        return Ok(Some(group));
                    }
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
            .map_err(to_py_err)?;
        Ok(group.map(Group::from))
    }

    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Group> {
        loop {
            if let Some(group) = self.next_group(py, 100)? {
                return Ok(group);
            }
            py.check_signals()?;
        }
    }

    fn close(&mut self) -> PyResult<()> {
        self.receiver.stop().map_err(to_py_err)
    }
}

#[pymodule]
fn device_system(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DeviceSystem>()?;
    m.add_class::<AlignedSystem>()?;
    m.add_class::<Frame>()?;
    m.add_class::<Group>()?;
    Ok(())
}
