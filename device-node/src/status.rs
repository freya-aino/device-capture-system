use std::{
    io::Cursor,
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};

use anyhow::{Error, Result, anyhow};
use shared::{ConnectionStatus, Device, DeviceStatus};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::{ControlError, DeviceAction, DeviceNode};

pub const DEFAULT_STATUS_ADDR: &str = "127.0.0.1:8080";

pub fn bind_status_server(addr: &str) -> Result<Server, Error> {
    Server::http(addr).map_err(|e| anyhow!("failed to bind status server on {}: {}", addr, e))
}

pub fn start_status_server<T>(
    server: Server,
    node: Arc<Mutex<DeviceNode<T>>>,
) -> JoinHandle<Result<(), Error>>
where
    T: Device + Send + 'static,
    T::Config: Clone + Send + 'static,
{
    spawn(move || -> Result<(), Error> {
        loop {
            let request = server.recv()?;
            if let Err(e) = handle_request(request, &node) {
                tracing::warn!("Status request failed: {}", e);
            }
        }
    })
}

pub fn status_body(device_status: &DeviceStatus, connection_status: &ConnectionStatus) -> String {
    format!(
        "{{\"device\":\"{:?}\",\"connection\":\"{:?}\"}}",
        device_status, connection_status
    )
}

fn handle_request<T>(request: Request, node: &Mutex<DeviceNode<T>>) -> Result<(), Error>
where
    T: Device,
    T::Config: Clone,
{
    let path = request.url().to_string();
    let is_get = *request.method() == Method::Get;
    let is_post = *request.method() == Method::Post;
    let action = match (is_get, is_post, path.as_str()) {
        (true, _, "/status") => None,
        (_, true, "/start") => Some(DeviceAction::Start),
        (_, true, "/stop") => Some(DeviceAction::Stop),
        (_, true, "/restart") => Some(DeviceAction::Restart),
        _ => {
            request.respond(Response::from_string("not found").with_status_code(404))?;
            return Ok(());
        }
    };

    let mut guard = match node.lock() {
        Ok(guard) => guard,
        Err(_) => return respond_unavailable(request),
    };
    let control = match action {
        Some(action) => guard.apply(action),
        None => Ok(()),
    };
    let status = guard.status();
    drop(guard);

    match control {
        Err(ControlError::InvalidTransition {
            action,
            status: current,
        }) => {
            let body = format!(
                "{{\"error\":\"cannot {:?} while device is {:?}\"}}",
                action, current
            );
            request.respond(json_response(body, 409))?;
        }
        Err(ControlError::Failed(e)) => {
            tracing::warn!("Device control failed: {}", e);
            request.respond(json_response(
                "{\"error\":\"device control failed\"}".to_string(),
                500,
            ))?;
        }
        Ok(()) => match status {
            Ok((device_status, connection_status)) => {
                let body = status_body(&device_status, &connection_status);
                request.respond(json_response(body, 200))?;
            }
            Err(e) => {
                tracing::warn!("Status unavailable: {}", e);
                return respond_unavailable(request);
            }
        },
    }
    Ok(())
}

fn respond_unavailable(request: Request) -> Result<(), Error> {
    request.respond(json_response(
        "{\"error\":\"status unavailable\"}".to_string(),
        500,
    ))?;
    Ok(())
}

fn json_response(body: String, status_code: u16) -> Response<Cursor<Vec<u8>>> {
    let content_type = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("static content-type header is valid");
    Response::from_string(body)
        .with_status_code(status_code)
        .with_header(content_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_body_reports_both_states() {
        let body = status_body(&DeviceStatus::Running, &ConnectionStatus::Active);
        assert_eq!(body, "{\"device\":\"Running\",\"connection\":\"Active\"}");
    }
}
