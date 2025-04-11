use anyhow::{Error, Result};

use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use webrtc::data_channel::RTCDataChannel;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::media;


// fn generate_random_image(w: u32, h: u32, c: u32) -> Vec<u8> {
//     let size = (w * h * c) as usize;
//     let arr = vec![0u8; size];
//     arr
// }

pub enum ConnectionStatus {
    Initialized,
    Active,
    Closed,
}

// trait Process {
//     fn initialize(&self) -> Result<(), Error>;
//     fn start(&self) -> Result<(), Error>;
//     fn stop(&self) -> Result<(), Error>;
//     fn pause(&self) -> Result<(), Error>;
//     fn resume(&self) -> Result<(), Error>;
//     fn restart(&self) -> Result<(), Error>;
//     fn get_stats(&self) -> CaptureStats;
// }



// #[derive(Debug)]
// pub struct CaptureStats {
//     frames_captured: u64,
//     frames_droped: u64,
//     bytes_captured: u64,
//     current_fps: f32,
//     current_bitrate: f32,
//     current_latency: f32,
//     uptime_seconds: u64,
//     last_frame_time: SystemTime,
// }

// #[derive(Debug)]
// pub struct CaptureProcess {
//     device: Device,
//     process_id: u32,
//     port: u32,
//     start_time: SystemTime,
//     stats: CaptureStats,
// }


// pub struct ConnectionInformation {
//     packets_sent: u64,
//     packets_received: u64,
//     dropped_packets: u64,
//     latency: u64,
// }

// pub struct Connection {
//     sender_ip: String,
//     sender_port: u16,
//     receiver_ip: String,
//     receiver_port: u16,
//     connection_status: ConnectionStatus,
//     connection_information: ConnectionInformation,
// }

pub struct ConnectionManager {
    address: SocketAddrV4,
    information_channel: Arc<RTCDataChannel>,
    frame_data_channel: Arc<RTCDataChannel>,
    peer_connection: Arc<RTCPeerConnection>,
    connection_status: ConnectionStatus,
}


impl ConnectionManager {

    pub async fn new(address: Ipv4Addr, port: u16) -> Self {
        
        // create peer connection
        let media_engine = MediaEngine::default();
        let api = APIBuilder::new().with_media_engine(media_engine).build();
        let config = RTCConfiguration::default();
        let peer_connection = api.new_peer_connection(config).await.unwrap();

        // create channels for data transfer
        let frame_data_channel = peer_connection.create_data_channel(
            "frames",
            None,
        ).await.unwrap();
        
        let information_channel = peer_connection.create_data_channel(
            "info",
            None,
        ).await.unwrap();

        ConnectionManager {
            address: SocketAddrV4::new(address, port),
            frame_data_channel: frame_data_channel,
            information_channel: information_channel,
            peer_connection: Arc::from(peer_connection),
            connection_status: ConnectionStatus::Initialized,
        }
    }
}




// #[derive(Debug)]
// pub struct FramePacket {
//     frame: Vec<u8>,
//     device_information: DeviceInformation,
//     timestamp: u64,
//     frame_shape: Vec<u16>,
// }

// pub struct Connection {
//     ip: String,
//     port: String,
// }

// pub struct WebRTCManager {
//     peer_connection: Arc<RTCPeerConnection>,
//     data_channel: Option<Arc<RTCDataChannel>>,
// }





