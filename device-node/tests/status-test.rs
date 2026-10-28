use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use anyhow::{Error, Result};
use device_node::{DeviceNode, bind_status_server, start_status_server};
use shared::{Device, DeviceConfig, DeviceInformation, DeviceStatus, DeviceType, FramePacket};

struct StatusMockDevice {
    info: DeviceInformation,
}

struct StatusMockConfig;
impl DeviceConfig for StatusMockConfig {}

impl Device for StatusMockDevice {
    type Config = StatusMockConfig;

    fn id(&self) -> &str {
        &self.info.id
    }
    fn name(&self) -> &str {
        &self.info.name
    }
    fn device_type(&self) -> &DeviceType {
        &self.info.device_type
    }
    fn device_status(&self) -> &DeviceStatus {
        &self.info.device_status
    }
    fn get_configs(&self) -> Result<Vec<Self::Config>, Error> {
        Ok(vec![StatusMockConfig])
    }
    fn start(
        &mut self,
        _config: Self::Config,
        _callback: Box<dyn Fn(FramePacket) -> Result<(), Error> + Send + 'static>,
    ) -> Result<std::thread::JoinHandle<Result<(), Error>>, Error> {
        Err(anyhow::anyhow!("not used in status test"))
    }
    fn stop(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        path
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn status_endpoint_reports_device_and_connection_state() {
    let device = StatusMockDevice {
        info: DeviceInformation {
            id: "status-mock".to_string(),
            name: "status-mock".to_string(),
            device_type: DeviceType::Camera,
            device_status: DeviceStatus::Available,
        },
    };
    let node = DeviceNode::new_endpoint(device, "inproc://status-test-unused", 10, 1024);
    let node = Arc::new(Mutex::new(node));

    let server = bind_status_server("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let _handle = start_status_server(server, node);

    let ok = http_get(port, "/status");
    assert!(ok.starts_with("HTTP/1.1 200"), "unexpected response: {}", ok);
    assert!(ok.contains("application/json"), "missing content type: {}", ok);
    assert!(
        ok.contains("{\"device\":\"Available\",\"connection\":\"Available\"}"),
        "unexpected body: {}",
        ok
    );

    let missing = http_get(port, "/other");
    assert!(missing.starts_with("HTTP/1.1 404"), "unexpected response: {}", missing);
}

fn http_request(port: u16, method: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        method, path
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn status_device(id: &str) -> StatusMockDevice {
    StatusMockDevice {
        info: DeviceInformation {
            id: id.to_string(),
            name: id.to_string(),
            device_type: DeviceType::Microphone,
            device_status: DeviceStatus::Stopped,
        },
    }
}

#[test]
fn status_endpoint_rejects_non_get_methods() {
    let node = DeviceNode::new_endpoint(
        status_device("status-method"),
        "inproc://status-test-method",
        10,
        1024,
    );
    let node = Arc::new(Mutex::new(node));

    let server = bind_status_server("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let _handle = start_status_server(server, node);

    let response = http_request(port, "POST", "/status");
    assert!(response.starts_with("HTTP/1.1 404"), "unexpected response: {}", response);
}

#[test]
fn status_endpoint_reports_reflect_device_status_changes() {
    let node = DeviceNode::new_endpoint(
        status_device("status-stopped"),
        "inproc://status-test-stopped",
        10,
        1024,
    );
    let node = Arc::new(Mutex::new(node));

    let server = bind_status_server("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let _handle = start_status_server(server, node);

    let response = http_get(port, "/status");
    assert!(response.starts_with("HTTP/1.1 200"), "unexpected response: {}", response);
    assert!(
        response.contains("{\"device\":\"Stopped\",\"connection\":\"Available\"}"),
        "unexpected body: {}",
        response
    );
}

#[test]
fn status_endpoint_returns_500_when_node_lock_is_poisoned() {
    let node = DeviceNode::new_endpoint(
        status_device("status-poisoned"),
        "inproc://status-test-poisoned",
        10,
        1024,
    );
    let node = Arc::new(Mutex::new(node));

    let poisoner = node.clone();
    let join = std::thread::spawn(move || {
        let _guard = poisoner.lock().unwrap();
        panic!("poison the node lock for the test");
    })
    .join();
    assert!(join.is_err());
    assert!(node.is_poisoned());

    let server = bind_status_server("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let _handle = start_status_server(server, node);

    let response = http_get(port, "/status");
    assert!(response.starts_with("HTTP/1.1 500"), "unexpected response: {}", response);
    assert!(
        response.contains("{\"error\":\"status unavailable\"}"),
        "unexpected body: {}",
        response
    );
}
