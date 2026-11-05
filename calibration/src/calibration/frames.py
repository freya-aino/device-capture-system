"""Decode `device_system.Frame` camera payloads into BGR images.

The wire carries the camera's native format (`pixel_format`): `MJPG` is one JPEG per frame, `YUYV` is
two bytes per pixel. Request an uncompressed mode with `DEVICE_NODE_CAMERA_WIDTH=640 ...` if you want
to avoid the JPEG decode.
"""

import cv2
import numpy as np


def decode_camera_frame(frame) -> np.ndarray:
    return decode_payload(frame.data, frame.pixel_format, frame.frame_shape)


def decode_payload(data: bytes, pixel_format: str, frame_shape) -> np.ndarray:
    width, height = frame_shape[0], frame_shape[1]
    buf = np.frombuffer(data, dtype=np.uint8)
    if pixel_format == "MJPG":
        image = cv2.imdecode(buf, cv2.IMREAD_COLOR)
        if image is None:
            raise ValueError("MJPG payload is not a decodable JPEG")
        return image
    if pixel_format == "YUYV":
        if buf.size != width * height * 2:
            raise ValueError(f"YUYV payload is {buf.size} bytes, expected {width * height * 2}")
        return cv2.cvtColor(buf.reshape(height, width, 2), cv2.COLOR_YUV2BGR_YUY2)
    raise ValueError(f"unsupported pixel format {pixel_format!r}")


def camera_images(device_system, device_id: str):
    """Yield decoded images from one camera of a connected `DeviceSystem`."""
    for frame in device_system:
        if frame.device_id == device_id and frame.device_type == "Camera":
            yield decode_camera_frame(frame)
