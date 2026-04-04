import json
import pytest
import numpy as np

from pydantic import ValidationError
from datetime import datetime

from device_capture_system import deviceIO


def test_get_all_devices_ffmpeg():
    devices = deviceIO.get_all_devices_ffmpeg()
    assert len(devices) > 0


def test_save_periphery_devices_to_config(tmp_path):
    mock_devices = [
        deviceIO.PeripheryDevice(device_id="device1", name="Device 1"), 
        deviceIO.PeripheryDevice(device_id="device2", name="Device 2")
    ]
    
    test_config_file = tmp_path / "./test_config.json"
    deviceIO.save_periphery_devices_to_config(mock_devices, test_config_file)

    # Step 3: Open the test config file and load the JSON data
    with open(test_config_file, "r") as f:
        loaded_data = json.load(f)

    # Step 4: Assert that the loaded data matches the mock list data
    assert loaded_data == [device.model_dump() for device in mock_devices]


@pytest.fixture
def mock_devices_file(tmp_path):
    data = [
        {
            "device_id": "device1",
            "name": "Device 1",
            "device_type": "video",
            "width": 1920,
            "height": 1080,
            "fps": 30,
            "pixel_format": "rgb24"
            
        },
        {
            "device_id": "device2",
            "name": "Device 2",
            "device_type": "audio",
            "channels": 1,
            "sample_rate": 16000,
            "sample_size": 16,
        },
    ]
    file = tmp_path / "devices.json"
    with open(file, 'w') as f:
        json.dump(data, f)
    return str(file)

def test_load_all_devices_from_config(mock_devices_file):
    
    # test load camera devices
    camera_devices = deviceIO.load_all_devices_from_config("video", mock_devices_file)
    
    # test load audio devices
    audio_devices = deviceIO.load_all_devices_from_config("audio", mock_devices_file)
    