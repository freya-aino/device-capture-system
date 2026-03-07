# Device Capture System

## TODO
- [ ] Device Node CLI
  - [ ] Generate / Save Config
- [ ] Device Node HTTP Server
  - [ ] Device Status
  - [ ] StartDevice / StopDevice / RestartDevice
- [ ] Control Hub 
  - [ ] Load Config and Initialize ZMQ
  - [ ] Sync
  - [ ] Calibrate
    - [ ] choose & implement general reference point detection model
    - [ ] transfere calibration code from python-opencv
- [ ] Client
  - [ ] write python client library
    - [ ] DeviceSystem(hub_ip, hub_port)
    - [ ] DeviceSystem.get_frame() -> generator 

## Device Node

- Device Code running `alsa` and `v4l2` to capture devices.
- Open ZMQ Socket for transmitting raw video/audio data
- REST API for querying (device parameters) and Controling device capture

## Descriptio

## Device Setup

## Usage
