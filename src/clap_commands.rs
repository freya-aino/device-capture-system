use clap::{Parser, Subcommand, ValueEnum};


#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List,
    Camera {
        // device command
        #[clap(short, long)]
        index: u8,

        #[command(subcommand)]
        device_command: DeviceCommand,
    },
    Microphone {
        // device command
        #[clap(short, long)]
        index: u8,

        #[command(subcommand)]
        device_command: DeviceCommand,
    },
}

#[derive(Subcommand)]
pub enum DeviceCommand {
    ListConfigs,
    Info,
    Start,
    Stop,
    Pause,
    Resume,
    Terminate,
}


// #[derive(Parser)]
// #[command(author, version, about, long_about = None)]
// struct Args {


//     // // start capture, device_type and index are required
//     // #[clap(short, long)]
//     // capture: bool,

//     // list configs
//     #[clap(short='c', long)]


//     // device type to capture
//     #[clap(short='t', long, value_enum)]
//     device_type: Option<DeviceType>,

//     // device index to capture, to find index run `--list-devices`
//     #[clap(short, long)]
//     index: Option<usize>,
// }
