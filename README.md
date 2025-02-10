# Device Capture System

## Prerequisites
- make sure to have `ffmpeg` installed on the device
- `ffmpeg` should be available to the commandline inside the project

## Description

A simple system to capture ffmpeg input devices (cameras and microphones), **currently only using dshow-capture on windows**.

## Environment Setup
### 1) clone the repo and cd into directory
```shell
git clone https://github.com/kristina-aino/device-capture-system
cd device-capture-system
```
---
### 2) poetry setup, test and build
- activate an environment that has `poetry` installed.
```shell
poetry install
poetry run python -m pytest
```

## Device Setup
#### Print all devices to check
```shell
python ./device_helper.py -p
```
#### Configure all devices (set output directory with `-o`, default: `./configs/devices.json`)
```shell
python ./device_helper.py -c -o ./configs/raw_devices.json
```
#### Example device list produced by `device_helper`
```json
[
  {
    "device_id": "@device_pnp_\\\\?\\usb#vid_5986&pid_1195&mi_00#6&1f473198&1&0000#{65e8773d-8f56-11d0-a3b9-00a0c9223196}\\global",
    "name": "Integrated Camera",
    "device_type": "video",
    "width": 2560,
    "height": 1440,
    "fps": 30.0,
    "pixel_format": "nv12"
  },
  {
    "device_id": "@device_cm_{33D9A762-90C8-11D0-BD43-00A0C911CE86}\\wave_{63BF8555-1DE6-47A2-83D7-788AF629CE5F}",
    "name": "Mikrofonarray (Intel\u00c2\u00ae Smart Sound Technologie f\u00c3\u00bcr digitale Mikrofone)",
    "device_type": "audio",
    "channels": 1,
    "sample_rate": 32000,
    "sample_size": 16
  }
]
```

## Usage
### Test the video stream
```shell
poetry run python ./test_video_stream.py --num_frames 600
```
### Test the audio stream
```shell
poetry run python ./test_audio_stream.py
```
### Save a video
```shell
poetry run python ./save_video.py --output_path ./saved_videos/
```
### Save some images
```shell
poetry run python ./save_images.py --output_path ./saved_images/ --num_images 100
```
