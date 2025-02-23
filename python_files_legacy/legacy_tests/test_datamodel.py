import pytest
from pydantic import ValidationError
from datetime import datetime
import numpy as np

import device_capture_system.datamodel as datamodel


@pytest.fixture
def periphery_device():
    return datamodel.PeripheryDevice(
        device_id="uuid",
        name="test_device",
        device_type=None
    )

@pytest.fixture
def frame():
    return np.ndarray([1, 2, 3])

@pytest.fixture
def media_file():
    return datamodel.MediaFile(
        file_path="path",
        file_name="name",
        file_extension="ext"
    )

# ---------- DEVICE CLASSES ----------

def test_camera_device(periphery_device):
    datamodel.CameraDevice(
        **periphery_device.model_dump(),
        width=640,
        height=480,
        fps=30,
        pixel_format="rgb24"
    )

def test_audio_device(periphery_device):
    datamodel.AudioDevice(
        **periphery_device.model_dump(),
        sample_rate=44100,
        channels=1,
        sample_size=16,
    )

# ---------- MEDIA CLASSES ----------

def test_video_file(media_file):
    datamodel.VideoFile(
        **media_file.model_dump(),
        width=640,
        height=480,
        fps=30,
        seconds=10,
        codec="mp4v"
    )

def test_image_file(media_file):
    datamodel.ImageFile(
        **media_file.model_dump(),
        jpg_quality=100,
        png_compression=0
    )

# ---------- FRAME PACKET ----------

def test_frame_packet_initialization(periphery_device, frame):
    datamodel.FramePacket(
        device=periphery_device,
        frame=frame,
        start_read_dt=datetime.now(),
        end_read_dt=datetime.now()
    )

def test_frame_packet_dump(periphery_device, frame):
    frame_packet = datamodel.FramePacket(
        device=periphery_device,
        frame=frame,
        start_read_dt=datetime.now(),
        end_read_dt=datetime.now()
    )
    
    # chek if all keys are present
    fram_packet_dump = frame_packet.dump()
    assert "frame" in fram_packet_dump
    assert "data" in fram_packet_dump

    assert "start_read_timestamp" in fram_packet_dump["data"]
    assert "end_read_timestamp" in fram_packet_dump["data"]
    assert "device" in fram_packet_dump["data"]
    assert "device" in fram_packet_dump["data"]

    assert "type" in fram_packet_dump["data"]["device"]
    assert "parameters" in fram_packet_dump["data"]["device"]
    
    assert "shape" in fram_packet_dump["data"]["frame"]
    assert "dtype" in fram_packet_dump["data"]["frame"]
    
    # check if data is correct
    assert (fram_packet_dump["frame"] == frame).all()

