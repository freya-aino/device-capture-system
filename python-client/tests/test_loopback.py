import socket
import subprocess
import time
from pathlib import Path

import pytest

from device_system import DeviceSystem

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
HUB_CONSUMER_PORT = 5556


def wait_for_port(port, timeout=60.0):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            if sock.connect_ex(("127.0.0.1", port)) == 0:
                return
        time.sleep(0.1)
    raise TimeoutError(f"port {port} did not open within {timeout}s")


@pytest.fixture
def loopback_publisher():
    process = subprocess.Popen(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "device-system-py",
            "--example",
            "loopback-publisher",
        ],
        cwd=WORKSPACE_ROOT,
    )
    try:
        wait_for_port(HUB_CONSUMER_PORT)
        yield process
    finally:
        process.kill()
        process.wait()


def test_device_system_receives_frames_from_loopback_hub(loopback_publisher):
    ds = DeviceSystem("127.0.0.1", HUB_CONSUMER_PORT)
    try:
        frame = None
        deadline = time.monotonic() + 30.0
        while frame is None and time.monotonic() < deadline:
            frame = ds.next_frame(timeout_ms=500)

        assert frame is not None, "no frame received from the loopback hub"
        assert frame.device_id == "loopback-camera"
        assert frame.device_type == "camera"
        assert frame.frame_shape == [2, 2]
        assert frame.pixel_format == "YUYV"
        assert frame.sample_rate is None
        assert len(frame.data) == 4
    finally:
        ds.close()
