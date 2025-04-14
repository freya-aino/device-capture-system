
use std::time::Duration;

use clap::Parser;
use clap_commands::{Cli, Commands, DeviceCommand};

use devices::{CameraDevice, Device, DeviceManager, DeviceType, MicrophoneDevice};

use anyhow::{Error, Result};

use tokio::time::sleep;


// ---------- main function --------- //

#[tokio::main]
async fn main() -> Result<(), Error> {

    // let signaling_server = tokio::spawn(run_signaling_server("127.0.0.1".to_string(), "10000".to_string()));
    // let server_abort = signaling_server.abort_handle();

    // sleep(Duration::from_millis(1000)).await;

    // let sender = tokio::spawn(create_sender("127.0.0.1".to_string(), "10000".to_string()));

    // sleep(Duration::from_millis(4000)).await;


    // let server_timeout = tokio::time::timeout(Duration::from_secs(10), signaling_server);
    // let sender_timeout = tokio::time::timeout(Duration::from_secs(10), sender);


    // let (server_result, sender_result) = tokio::join!(
    //     sender_timeout,
    //     server_timeout
    // );

    // match server_result {
    //     Ok(_) => println!("Signaling server finished successfully."),
    //     Err(_) => {
    //         println!("Signaling server timed out.");
    //         server_abort.abort();
    //     },
    // }
    // match sender_result {
    //     Ok(_) => println!("Sender finished successfully."),
    //     Err(_) => println!("Sender timed out."),
    // }


    Ok(())    
}
