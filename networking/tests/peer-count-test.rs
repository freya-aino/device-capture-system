use std::thread::sleep;
use std::time::{Duration, Instant};

use networking::{Proxy, Sender};

fn wait_for(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        sleep(Duration::from_millis(20));
    }
    condition()
}

#[test]
fn peer_counts_track_producers_and_consumers() {
    let context = zmq::Context::new();
    let producer_a_endpoint = "tcp://127.0.0.1:16401".to_string();
    let producer_b_endpoint = "tcp://127.0.0.1:16402".to_string();
    let consumer_endpoint = "tcp://127.0.0.1:16403".to_string();

    let mut hub = Proxy::new_endpoints(
        producer_a_endpoint.clone(),
        consumer_endpoint.clone(),
        10,
        1024,
    );
    hub.add_from_endpoint(producer_b_endpoint.clone());
    let peers = hub.peer_counts();
    hub.start(&context).unwrap();

    assert_eq!(peers.producers(), 0);
    assert_eq!(peers.consumers(), 0);

    let mut producer_a = Sender::new_endpoint(producer_a_endpoint, 10, 1024);
    producer_a.start(&context).unwrap();
    let mut producer_b = Sender::new_endpoint(producer_b_endpoint, 10, 1024);
    producer_b.start(&context).unwrap();

    let consumer = context.socket(zmq::SUB).unwrap();
    consumer.set_subscribe(b"").unwrap();
    consumer.connect(&consumer_endpoint).unwrap();

    assert!(
        wait_for(Duration::from_secs(3), || peers.producers() == 2),
        "expected 2 producers, saw {}",
        peers.producers()
    );
    assert!(
        wait_for(Duration::from_secs(3), || peers.consumers() == 1),
        "expected 1 consumer, saw {}",
        peers.consumers()
    );

    drop(consumer);
    assert!(
        wait_for(Duration::from_secs(3), || peers.consumers() == 0),
        "consumer count did not drop after disconnect, saw {}",
        peers.consumers()
    );
    assert_eq!(peers.producers(), 2);

    producer_a.stop().unwrap();
    producer_b.stop().unwrap();
    hub.stop().unwrap();
}
