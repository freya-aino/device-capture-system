import json
import pytest
import numpy as np
import concurrent.futures as concurrent_future

from unittest.mock import MagicMock, patch
from pydantic import ValidationError
from datetime import datetime

from device_capture_system import deviceIO
from device_capture_system import datamodel


@pytest.fixture
def ffmpeg_reader():
    class MockFFMPEGReader(deviceIO.FFMPEGReader):
        def start(self, file, options):
            super().start(file, options)
        def read(self):
            return super().read()
    
    device = datamodel.PeripheryDevice(device_id="device1", name="Device 1", device_type="video")
    return MockFFMPEGReader(device, 'test_logger')

def test_is_active(ffmpeg_reader):
    assert not ffmpeg_reader.is_active()
    ffmpeg_reader.container = MagicMock()
    assert ffmpeg_reader.is_active()


def test_stop(ffmpeg_reader):
    ffmpeg_reader.container = MagicMock()
    ffmpeg_reader.stream = MagicMock()
    ffmpeg_reader.stop()
    assert ffmpeg_reader.container is None
    assert ffmpeg_reader.stream is None

@patch('device_capture_system.deviceIO.av.open')
def test_start(mock_av_open, ffmpeg_reader):
    
    ffmpeg_reader.start('file_string', {'option': 'value'})
    mock_av_open.assert_called_once_with(file='file_string', format='dshow', options={'option': 'value'})
    
    assert ffmpeg_reader.container is not None
    assert ffmpeg_reader.stream is not None

def test_read(ffmpeg_reader):
    ffmpeg_reader.container = MagicMock()
    ffmpeg_reader.stream = MagicMock()
    ffmpeg_reader.container.decode = MagicMock()
    
    ittr_return = MagicMock()
    ittr_return.to_ndarray.return_value = np.array([[1, 2, 3]])
    decode_iter = iter([ittr_return])
    
    ffmpeg_reader.container.decode.return_value = decode_iter
    
    # test normal case
    ret = ffmpeg_reader.read()
    assert isinstance(ret, datamodel.FramePacket)
    assert (ret.frame == ittr_return.to_ndarray()).all(), f"ret={ret.frame}, ittr_return.to_ndarray()={ittr_return.to_ndarray()}"
    
    # test StopIteration
    ret = ffmpeg_reader.read()
    assert ret is None
    
    # test TimeoutError
    ret_mm = MagicMock(return_value=iter([MagicMock()]))
    ret_mm.__next__.side_effect = concurrent_future.TimeoutError
    ffmpeg_reader.container.decode.return_value = ret_mm
    ret = ffmpeg_reader.read()
    assert ret is None