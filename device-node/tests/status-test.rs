use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use anyhow::{Error, Result};
use device_node::{DeviceNode, bind_status_server, start_status_server};
use shared::{Device, DeviceConfig, DeviceInformation, DeviceStatus, DeviceType, FramePacket};

struct StatusMockDevice {
    info: DeviceInformation,
}

#[derive(Clone)]
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
        self.info.device_status = DeviceStatus::Running;
        Ok(std::thread::spawn(|| -> Result<(), Error> { Ok(()) }))
    }
    fn stop(&mut self) -> Result<(), Error> {
        self.info.device_status = DeviceStatus::Stopped;
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
    assert!(
        ok.starts_with("HTTP/1.1 200"),
        "unexpected response: {}",
        ok
    );
    assert!(
        ok.contains("application/json"),
        "missing content type: {}",
        ok
    );
    assert!(
        ok.contains("{\"device\":\"Available\",\"connection\":\"Available\"}"),
        "unexpected body: {}",
        ok
    );

    let missing = http_get(port, "/other");
    assert!(
        missing.starts_with("HTTP/1.1 404"),
        "unexpected response: {}",
        missing
    );
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
    assert!(
        response.starts_with("HTTP/1.1 404"),
        "unexpected response: {}",
        response
    );
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
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected response: {}",
        response
    );
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
    assert!(
        response.starts_with("HTTP/1.1 500"),
        "unexpected response: {}",
        response
    );
    assert!(
        response.contains("{\"error\":\"status unavailable\"}"),
        "unexpected body: {}",
        response
    );
}

fn http_post(port: u16, path: &str) -> String {
    http_request(port, "POST", path)
}

fn started_node(id: &str, endpoint: &str) -> Arc<Mutex<DeviceNode<StatusMockDevice>>> {
    let context = zmq::Context::new();
    let mut node = DeviceNode::new_endpoint(status_device(id), endpoint.to_string(), 10, 1024);
    node.start(StatusMockConfig, &context).unwrap();
    Arc::new(Mutex::new(node))
}

fn serve(
    node: Arc<Mutex<DeviceNode<StatusMockDevice>>>,
) -> (u16, std::thread::JoinHandle<Result<(), Error>>) {
    let server = bind_status_server("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    (port, start_status_server(server, node))
}

#[test]
fn remote_stop_then_start_then_restart_transition_the_device() {
    let node = started_node("remote-cycle", "tcp://127.0.0.1:16311");
    let (port, _handle) = serve(node);

    let stopped = http_post(port, "/stop");
    assert!(
        stopped.starts_with("HTTP/1.1 200"),
        "unexpected response: {}",
        stopped
    );
    assert!(
        stopped.contains("{\"device\":\"Stopped\",\"connection\":\"Active\"}"),
        "{}",
        stopped
    );

    let started = http_post(port, "/start");
    assert!(
        started.starts_with("HTTP/1.1 200"),
        "unexpected response: {}",
        started
    );
    assert!(
        started.contains("{\"device\":\"Running\",\"connection\":\"Active\"}"),
        "{}",
        started
    );

    let restarted = http_post(port, "/restart");
    assert!(
        restarted.starts_with("HTTP/1.1 200"),
        "unexpected response: {}",
        restarted
    );
    assert!(
        restarted.contains("{\"device\":\"Running\",\"connection\":\"Active\"}"),
        "{}",
        restarted
    );
}

#[test]
fn remote_start_while_running_returns_409() {
    let node = started_node("remote-start-running", "tcp://127.0.0.1:16312");
    let (port, _handle) = serve(node);

    let response = http_post(port, "/start");
    assert!(
        response.starts_with("HTTP/1.1 409"),
        "unexpected response: {}",
        response
    );
    assert!(
        response.contains("cannot Start while device is Running"),
        "{}",
        response
    );
}

#[test]
fn remote_stop_while_stopped_returns_409() {
    let node = started_node("remote-stop-stopped", "tcp://127.0.0.1:16313");
    let (port, _handle) = serve(node);
    assert!(http_post(port, "/stop").starts_with("HTTP/1.1 200"));

    let response = http_post(port, "/stop");
    assert!(
        response.starts_with("HTTP/1.1 409"),
        "unexpected response: {}",
        response
    );
    assert!(
        response.contains("cannot Stop while device is Stopped"),
        "{}",
        response
    );
}

#[test]
fn remote_restart_while_stopped_returns_409() {
    let node = started_node("remote-restart-stopped", "tcp://127.0.0.1:16314");
    let (port, _handle) = serve(node);
    assert!(http_post(port, "/stop").starts_with("HTTP/1.1 200"));

    let response = http_post(port, "/restart");
    assert!(
        response.starts_with("HTTP/1.1 409"),
        "unexpected response: {}",
        response
    );
}

#[test]
fn remote_start_on_never_started_node_returns_409() {
    let mut device = status_device("remote-never-started");
    device.info.device_status = DeviceStatus::Available;
    let node = Arc::new(Mutex::new(DeviceNode::new_endpoint(
        device,
        "tcp://127.0.0.1:16315",
        10,
        1024,
    )));
    let (port, _handle) = serve(node);

    let response = http_post(port, "/start");
    assert!(
        response.starts_with("HTTP/1.1 409"),
        "unexpected response: {}",
        response
    );
}

#[test]
fn remote_control_rejects_unknown_post_paths() {
    let node = started_node("remote-unknown", "tcp://127.0.0.1:16316");
    let (port, _handle) = serve(node);

    let response = http_post(port, "/shutdown");
    assert!(
        response.starts_with("HTTP/1.1 404"),
        "unexpected response: {}",
        response
    );
}
