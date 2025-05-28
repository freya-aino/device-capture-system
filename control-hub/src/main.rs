use anyhow::{Error, Result};
use bincode::{config, encode_to_vec};
use shared::{DeviceInformation, DeviceType};
use std::thread::sleep;
use std::time::Duration;
use tiny_http::{Request, Response};

fn resp_fn(req: Request) -> Result<(), Error> {
    let info = DeviceInformation {
        device_type: DeviceType::Camera,
        id: 10,
        name: "test".to_string(),
    };
    let data = encode_to_vec(info, config::standard()).unwrap();
    let resp = Response::from_data(data);

    req.respond(resp).unwrap();
    Ok(())
}

fn main() {
    //

    shared::start_static_http_server("127.0.0.1".to_string(), "8080".to_string(), resp_fn).unwrap();
    sleep(Duration::from_secs(100));
}
