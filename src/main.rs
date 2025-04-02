mod datamodel;

use alsa::ctl::DeviceIter;
use v4l::Device;
use v4l::Format;

fn main() {
    let format = Format::new(1920, 1080, v4l::FourCC::new(b"YUYV"));

    println!("{:?}", format);

    let device_0 = Device::new(0);
}
