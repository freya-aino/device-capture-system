use std::thread::{JoinHandle, spawn};

use anyhow::{Error, Result, anyhow};
use networking::PeerCounts;
use tiny_http::{Header, Method, Request, Response, Server};

pub const DEFAULT_HUB_STATUS_ADDR: &str = "127.0.0.1:8081";

pub fn bind_status_server(addr: &str) -> Result<Server, Error> {
    Server::http(addr).map_err(|e| anyhow!("failed to bind hub status server on {}: {}", addr, e))
}

pub fn start_status_server(server: Server, peers: PeerCounts) -> JoinHandle<Result<(), Error>> {
    spawn(move || -> Result<(), Error> {
        loop {
            let request = server.recv()?;
            if let Err(e) = handle_request(request, &peers) {
                tracing::warn!("Hub status request failed: {}", e);
            }
        }
    })
}

pub fn status_body(producers: usize, consumers: usize) -> String {
    format!("{{\"producers\":{},\"consumers\":{}}}", producers, consumers)
}

fn handle_request(request: Request, peers: &PeerCounts) -> Result<(), Error> {
    if *request.method() != Method::Get || request.url() != "/status" {
        request.respond(Response::from_string("not found").with_status_code(404))?;
        return Ok(());
    }
    let body = status_body(peers.producers(), peers.consumers());
    request.respond(json_response(body, 200))?;
    Ok(())
}

fn json_response(body: String, status_code: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    let content_type = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("static content-type header is valid");
    Response::from_string(body)
        .with_status_code(status_code)
        .with_header(content_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use networking::{Proxy, Sender};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    fn get(port: u16, path: &str) -> String {
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
    fn status_body_reports_producer_and_consumer_counts() {
        assert_eq!(status_body(2, 1), "{\"producers\":2,\"consumers\":1}");
    }

    #[test]
    fn status_endpoint_reports_one_producer_and_one_consumer() {
        let context = zmq::Context::new();
        let producer_endpoint = "tcp://127.0.0.1:16411".to_string();
        let consumer_endpoint = "tcp://127.0.0.1:16412".to_string();

        let mut hub = Proxy::new_endpoints(
            producer_endpoint.clone(),
            consumer_endpoint.clone(),
            10,
            1024,
        );
        let peers = hub.peer_counts();
        hub.start(&context).unwrap();

        let server = bind_status_server("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let _status = start_status_server(server, peers);

        let mut producer = Sender::new_endpoint(producer_endpoint, 10, 1024);
        producer.start(&context).unwrap();
        let consumer = context.socket(zmq::SUB).unwrap();
        consumer.set_subscribe(b"").unwrap();
        consumer.connect(&consumer_endpoint).unwrap();

        let start = Instant::now();
        let mut response = get(port, "/status");
        while !response.ends_with("{\"producers\":1,\"consumers\":1}")
            && start.elapsed() < Duration::from_secs(3)
        {
            sleep(Duration::from_millis(20));
            response = get(port, "/status");
        }

        assert!(response.starts_with("HTTP/1.1 200"), "{}", response);
        assert!(response.contains("application/json"), "{}", response);
        assert!(
            response.ends_with("{\"producers\":1,\"consumers\":1}"),
            "{}",
            response
        );

        assert!(get(port, "/nope").starts_with("HTTP/1.1 404"));

        drop(consumer);
        producer.stop().unwrap();
        hub.stop().unwrap();
    }
}
