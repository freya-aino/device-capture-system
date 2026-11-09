use std::sync::atomic::{AtomicUsize, Ordering};

use networking::{Proxy, Receiver};
use shared::ConnectionStatus;

static ENDPOINT_ID: AtomicUsize = AtomicUsize::new(0);

fn unique_endpoint(name: &str) -> String {
    let id = ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
    format!("inproc://lifecycle-{}-{}", name, id)
}

#[test]
fn proxy_reports_available_before_start() {
    let proxy = Proxy::new_endpoints(
        unique_endpoint("proxy-before-from"),
        unique_endpoint("proxy-before-to"),
        10,
        1024,
    );

    let (from, to) = proxy.connection_status();
    assert_eq!(from, &ConnectionStatus::Available);
    assert_eq!(to, &ConnectionStatus::Available);
}

#[test]
fn proxy_reports_active_after_start_and_closed_after_stop() {
    let context = zmq::Context::new();
    let mut proxy = Proxy::new_endpoints(
        unique_endpoint("proxy-status-from"),
        unique_endpoint("proxy-status-to"),
        10,
        1024,
    );

    proxy.start(&context).unwrap();
    let (from, to) = proxy.connection_status();
    assert_eq!(from, &ConnectionStatus::Active);
    assert_eq!(to, &ConnectionStatus::Active);

    proxy.stop().unwrap();
    let (from, to) = proxy.connection_status();
    assert_eq!(from, &ConnectionStatus::Closed);
    assert_eq!(to, &ConnectionStatus::Closed);
}

#[test]
fn proxy_stop_without_start_is_a_no_op() {
    let mut proxy = Proxy::new_endpoints(
        unique_endpoint("proxy-nostart-from"),
        unique_endpoint("proxy-nostart-to"),
        10,
        1024,
    );

    assert!(proxy.stop().is_ok());
}

#[test]
fn proxy_reports_data_chunk_size_from_constructor() {
    let proxy = Proxy::new_endpoints(
        unique_endpoint("proxy-chunk-from"),
        unique_endpoint("proxy-chunk-to"),
        10,
        4096,
    );

    assert_eq!(proxy.data_chunk_size(), 4096);
}

#[test]
fn receiver_stop_without_start_returns_ok() {
    let mut receiver = Receiver::new_endpoint(unique_endpoint("receiver-stop"), 10, 1024);
    assert!(receiver.stop().is_ok());
    assert_eq!(receiver.connection_status(), &ConnectionStatus::Available);
}

#[test]
fn receiver_reports_data_chunk_size_and_no_drops_initially() {
    let receiver = Receiver::new_endpoint(unique_endpoint("receiver-init"), 10, 2048);

    assert_eq!(receiver.data_chunk_size(), 2048);
    assert_eq!(receiver.dropped_frames(), 0);
    assert_eq!(receiver.connection_status(), &ConnectionStatus::Available);
}

#[test]
#[should_panic(expected = "socket is None")]
fn receive_before_start_panics() {
    let mut receiver = Receiver::new_endpoint(unique_endpoint("receiver-unstarted"), 10, 1024);
    let _ = receiver.receive(zmq::DONTWAIT);
}

#[test]
#[should_panic(expected = "connection is closed")]
fn receive_after_stop_panics() {
    let context = zmq::Context::new();
    let mut receiver = Receiver::new_endpoint(unique_endpoint("receiver-closed"), 10, 1024);
    receiver.start(&context).unwrap();
    receiver.stop().unwrap();
    let _ = receiver.receive(zmq::DONTWAIT);
}

#[test]
fn receive_on_empty_socket_returns_none() {
    let context = zmq::Context::new();
    let mut receiver = Receiver::new_endpoint(unique_endpoint("receiver-empty"), 10, 1024);
    receiver.start(&context).unwrap();

    assert!(receiver.receive(zmq::DONTWAIT).unwrap().is_none());
    assert_eq!(receiver.dropped_frames(), 0);

    receiver.stop().unwrap();
}
