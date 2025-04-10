mod devices;
mod clap_commands;


use std::sync::Arc;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use clap::Parser;
use clap_commands::{Cli, Commands, DeviceCommand};

use devices::{CameraDevice, Device, DeviceManager, DeviceType, MicrophoneDevice};

use anyhow::{Error, Result};

use serde::ser::SerializeMap;
use tokio::net::TcpListener;
use tokio::time::sleep;
use webrtc::data_channel::RTCDataChannel;
use webrtc::{self, peer_connection};
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;

use serde::{Serialize, Deserialize};
use serde_json::to_vec;

use rand::Rng;

// ---------- test implemenation of webrtc --------- //

fn generate_random_image(w: u32, h: u32, c: u32) -> Vec<u8> {
    let size = (w * h * c) as usize;
    let arr = vec![0u8; size];
    arr
}


#[derive(Serialize, Deserialize)]
enum DataType {
    Audio,
    Video,
}

#[derive(Serialize, Deserialize)]
struct DataPacket {
    frame: Vec<u8>,
    timestamp: u64,
    data_type: DataType,
}

async fn setup_peer_connection() -> Result<RTCPeerConnection, Error> {

    let media_engine = MediaEngine::default();

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .build();

    let config = RTCConfiguration::default();

    let peer_connection = api.new_peer_connection(config).await?;
    
    Ok(peer_connection)
}

async fn run_signaling_server(ip: String, port: String) -> Result<(), Error> {
    let conn_string = format!("{}:{}", ip, port);
    let listener = TcpListener::bind(conn_string).await?;
    println!("Signaling server running on {}:{}", ip, port);

    while let Ok((socket, _)) = listener.accept().await {
        println!("New connection from {:?}", socket.peer_addr()?);
    }

    Ok(())
}

async fn send_frame(dc: &Arc<RTCDataChannel>, data_packet: DataPacket) -> Result<(), Error> {

    let mut header = Vec::with_capacity(16);
    header.extend_from_slice(&data_packet.timestamp.to_be_bytes());
    header.extend_from_slice(&(data_packet.data_type as u8).to_be_bytes());
    header.extend_from_slice(&(data_packet.frame.len() as u32).to_be_bytes());

    Ok(())
}

async fn create_sender(ip: String, port: String) -> Result<()> {
    let api = APIBuilder::new()
        .build();

    let config = RTCConfiguration::default();
    let peer_connection = api.new_peer_connection(config).await?;

    let conn_string = format!("{}:{}", ip, port);
    let stream = TcpStream::connect(conn_string)?;
    

    let offer = peer_connection.create_offer(None).await?;
    peer_connection.set_local_description(offer.clone()).await?;

    // let data_channel = peer_connection.create_data_channel(
    //     "data-frames",
    //     None,
    // ).await?;

    for _ in 0..3 {
        let frame = generate_random_image(2560, 2560, 3);
        println!("Sending data packet with length {}...", frame.len());
        let msg = DataPacket {
            frame: frame,
            timestamp: 0,
            data_type: DataType::Video,
        };

        let serialized_msg = &to_vec(&msg)?;

        send_frame(
            &()[0],
            msg.clone()
        )

        sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
}


// ---------- main function --------- //

#[tokio::main]
async fn main() -> Result<(), Error> {


    let signaling_server = tokio::spawn(run_signaling_server("127.0.0.1".to_string(), "10000".to_string()));
    let server_abort = signaling_server.abort_handle();

    sleep(Duration::from_millis(1000)).await;

    let sender = tokio::spawn(create_sender("127.0.0.1".to_string(), "10000".to_string()));

    sleep(Duration::from_millis(4000)).await;


    let server_timeout = tokio::time::timeout(Duration::from_secs(10), signaling_server);
    let sender_timeout = tokio::time::timeout(Duration::from_secs(10), sender);


    let (server_result, sender_result) = tokio::join!(
        sender_timeout,
        server_timeout
    );

    match server_result {
        Ok(_) => println!("Signaling server finished successfully."),
        Err(_) => {
            println!("Signaling server timed out.");
            server_abort.abort();
        },
    }
    match sender_result {
        Ok(_) => println!("Sender finished successfully."),
        Err(_) => println!("Sender timed out."),
    }

    // let cli = Cli::parse();

    // let all_devices = DeviceManager::get_all_available_devices_managed()?;

    // match cli.command {
    //     Commands::List => {
    //         println!("\nDevices Found: {}", all_devices.len());
    //         for dev in all_devices.iter() {
    //             let type_str = match dev.device_info.device_type {
    //                 DeviceType::Camera => "Camera    ",
    //                 DeviceType::Microphone => "Microphone",
    //             };
    //             println!("{} : {} \t: {}", dev.system_id, type_str, dev.device_info.name);
    //         }
    //     }
    //     Commands::Camera { index, device_command } => {
    //         match device_command {
    //             DeviceCommand::Start => {

    //             }
    //             _ => {
    //                 println!("Camera command not implemented yet.");
    //             }
    //         }
    //     },
    //     Commands::Microphone { index, device_command } => {
    //         match device_command {
    //             DeviceCommand::Start => {

    //             }
    //             _ => {
    //                 println!("Microphone command not implemented yet.");
    //             }
    //         }
    //     }
    // }


    Ok(())    
}
