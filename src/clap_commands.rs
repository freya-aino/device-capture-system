use clap::{Parser, Subcommand};


#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List {
        #[clap(short, long)]
        cameras: bool,
        #[clap(short, long)]
        microphones: bool,
    },
    // Camera {
    //     #[clap(short, long)]
    //     id: Option<u8>,
    //     #[clap(short, long)]
    //     config_id: Option<u8>,

    //     #[command(subcommand)]
    //     action: Option<ActionCommand>,
    // },
    // Show,
    // Configs,
    // Info,
    // Start,
    // Stop,
    // Pause,
    // Resume,
    // Terminate,
}

// #[derive(Subcommand)]
// pub enum ActionCommands {
//     Start,
//     Stop,
//     Pause,
//     Resume,
//     Terminate,
// }


// #[derive(Parser)]
// #[command(author, version, about, long_about = None)]
// struct Args {

//     /// list devices
//     #[clap(short='d', long)]
//     list_devices: bool,

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
