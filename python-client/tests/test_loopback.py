import socket
import subprocess
import time
from pathlib import Path

import pytest

from device_system import AlignedSystem, DeviceSystem

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
HUB_CONSUMER_PORT = 5556
HUB_SYNC_PORT = 5557


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
        wait_for_port(HUB_SYNC_PORT)
        yield process
    finally:
        process.kill()
        process.wait()


def first(poll, deadline_s=30.0):
    item = None
    deadline = time.monotonic() + deadline_s
    while item is None and time.monotonic() < deadline:
        item = poll()
    return item


def test_device_system_receives_frames_from_loopback_hub(loopback_publisher):
    ds = DeviceSystem("127.0.0.1", HUB_CONSUMER_PORT)
    try:
        frame = first(lambda: ds.next_frame(timeout_ms=500))
        assert frame is not None, "no frame received from the loopback hub"
        assert frame.device_id in ("loopback-camera", "loopback-camera-2")
        assert frame.device_type == "camera"
        assert frame.frame_shape == [2, 1]
        assert frame.pixel_format == "YUYV"
        assert frame.sample_rate is None
        assert frame.capture_timestamp is not None
        assert len(frame.data) == 4

        width, height, channels, pixels = frame.image()
        assert (width, height, channels) == (2, 1, 3)
        assert pixels == bytes([0, 0, 0, 255, 255, 255]), "YUYV black and white become RGB"
    finally:
        ds.close()


def test_aligned_system_yields_groups_of_both_cameras(loopback_publisher):
    aligned = AlignedSystem("127.0.0.1", HUB_SYNC_PORT)
    try:
        group = first(lambda: aligned.next_group(timeout_ms=500))
        assert group is not None, "no aligned group received from the sync port"
        assert len(group) == 2
        assert sorted(f.device_id for f in group.frames) == [
            "loopback-camera",
            "loopback-camera-2",
        ]
        assert group.skew_s == 0.0, "both frames share one capture time"
        assert all(f.image()[2] == 3 for f in group.frames)
        second = next(iter(aligned))
        assert len(second.frames) == 2
    finally:
        aligned.close()

