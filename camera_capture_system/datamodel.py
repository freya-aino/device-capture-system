from abc import ABC
from typing_extensions import Annotated
from pydantic import BaseModel, Field, StrictStr, Strict, StrictInt
from dataclasses import dataclass
from numpy import ndarray, uint8, int16
from datetime import datetime
from numpy import ascontiguousarray

StrictNonEmptyStr = Annotated[StrictStr, Field(min_length=1), Strict()]

class Camera(BaseModel):
    uuid: StrictNonEmptyStr # unique identifier for the camera
    id: Annotated[StrictInt, Field(ge=0)] # index of the camera in the system (opencv index)
    publishing_port: Annotated[StrictInt, Field(ge=1025, le=65535)] # port to publish camera data to
    width: Annotated[StrictInt, Field(ge=640, le=3840)]
    height: Annotated[StrictInt, Field(ge=480, le=2160)]
    fps: Annotated[StrictInt, Field(ge=15, le=120)]
    name: StrictNonEmptyStr

class AudioDevice(BaseModel):
    uuid: StrictNonEmptyStr # unique identifier for the audio device
    id: Annotated[StrictInt, Field(ge=0)] # index of the audio device in the system (pyaudio index)
    publishing_port: Annotated[StrictInt, Field(ge=1025, le=65535)] # port to publish audio data to
    type: StrictNonEmptyStr
    sample_rate: Annotated[StrictInt, Field(ge=8000, le=192000)] # sample rate in Hz
    frames_per_buffer: Annotated[StrictInt, Field(ge=1)]
    channels: Annotated[StrictInt, Field(ge=1)]


class VideoParameters(BaseModel):
    save_path: StrictNonEmptyStr # Path to save the videos under save_path/videos
    fps: Annotated[StrictInt, Field(ge=15, le=120)]
    seconds: Annotated[StrictInt, Field(ge=1)] # Number of seconds in output video
    codec: Annotated[StrictStr, Field(min_length=4, max_length=4)] # Codec to use for the video
    output_format: StrictNonEmptyStr

class ImageParameters(BaseModel):
    save_path: StrictNonEmptyStr # Path to save the images under save_path/images
    jpg_quality: Annotated[StrictInt, Field(ge=0, le=100)]
    png_compression: Annotated[StrictInt, Field(ge=0, le=100)]
    output_format: StrictNonEmptyStr

@dataclass
class FramePacket(ABC):
    
    def dump(self):
        pass
    
    @classmethod
    def create(cls):
        pass


@dataclass
class AudioFramePacket(FramePacket):
    audio_device: AudioDevice
    audio_frame: ndarray[int16]
    start_read_dt: datetime
    end_read_dt: datetime
    
    # format data for imagezmq
    def dump(self) -> tuple[ndarray[int16], dict]:
        
        # check if frame is contiguous and convert to contiguous if not
        if not frame.flags["C_CONTIGUOUS"]:
            frame = ascontiguousarray(frame)
        
        return (
            self.audio_frame,
            {
                "audio_device": self.audio_device.model_dump(),
                "audio_data": {
                    "dtype": str(self.audio_frame.dtype),
                    "shape": self.audio_frame.shape
                },
                "start_read_timestamp": self.start_read_dt.timestamp(),
                "end_read_timestamp": self.end_read_dt.timestamp()
            }
        )
    
    # if data is from imagezmq
    @classmethod
    def create(cls, frame: ndarray[int16], data: dict):
        return cls(
            camera=Camera(**data["audio_device"]),
            audio_frame=frame,
            start_read_dt=datetime.fromtimestamp(data["start_read_timestamp"]),
            end_read_dt=datetime.fromtimestamp(data["end_read_timestamp"]))
        
    def __post_init__(self):
        assert isinstance(self.audio_device, AudioDevice), "camera must be a Camera object"
        assert isinstance(self.audio_frame, ndarray), "frame must be a numpy array"
        assert isinstance(self.start_read_dt, datetime), "start read datetime must be a datetime object"
        assert isinstance(self.end_read_dt, datetime), "end read datetime must be a datetime object"

@dataclass
class CameraFramePacket(FramePacket):
    camera: Camera
    camera_frame: ndarray[uint8]
    start_read_dt: datetime
    end_read_dt: datetime
    
    # format data for imagezmq
    def dump(self) -> tuple[ndarray[uint8], dict]:
        
        # check if frame is contiguous and convert to contiguous if not
        if not frame.flags["C_CONTIGUOUS"]:
            frame = ascontiguousarray(frame)
        
        return (
            self.camera_frame, 
            {
                "camera": self.camera.model_dump(),
                "image_data": {
                    "dtype": str(self.camera_frame.dtype),
                    "shape": self.camera_frame.shape
                },
                "start_read_timestamp": self.start_read_dt.timestamp(),
                "end_read_timestamp": self.end_read_dt.timestamp()
            }
        )
    
    # if data is from imagezmq
    @classmethod
    def create(cls, frame: ndarray[uint8], data: dict):
        return cls(
            camera=Camera(**data["camera"]),
            camera_frame=frame,
            start_read_dt=datetime.fromtimestamp(data["start_read_timestamp"]),
            end_read_dt=datetime.fromtimestamp(data["end_read_timestamp"]))
        
    def __post_init__(self):
        assert isinstance(self.camera, Camera), "camera must be a Camera object"
        assert isinstance(self.camera_frame, ndarray), "frame must be a numpy array"
        assert isinstance(self.start_read_dt, datetime), "start read datetime must be a datetime object"
        assert isinstance(self.end_read_dt, datetime), "end read datetime must be a datetime object"