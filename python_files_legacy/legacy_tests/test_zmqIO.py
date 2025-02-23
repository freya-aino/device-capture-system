import pytest
import numpy as np

from time import sleep
from pydantic import ValidationError
from datetime import datetime
from threading import Thread

import device_capture_system.zmqIO as zmqIO
import device_capture_system.datamodel as datamodel

@pytest.fixture
def zmq_sender():
    return zmqIO.ZMQSender(host="127.0.0.1", port=1025)
@pytest.fixture
def zmq_receiver():
    return zmqIO.ZMQReceiver(host="127.0.0.1", port=1025)
@pytest.fixture
def frame_packet():
    return datamodel.FramePacket(
        device=datamodel.PeripheryDevice(
            device_id="uuid",
            name="test_device",
        ),
        frame=np.ndarray([1, 2, 3]),
        start_read_dt=datetime.now(),
        end_read_dt=datetime.now()
    )

def test_zmq_sender_open_close(zmq_sender):
    zmq_sender.start()
    assert zmq_sender.is_active()
    zmq_sender.stop()
    assert not zmq_sender.is_active()

def test_zmq_receiver_open_close(zmq_receiver):
    zmq_receiver.start()
    assert zmq_receiver.is_active()
    zmq_receiver.stop()
    assert not zmq_receiver.is_active()

def test_zmq_sender_send(zmq_sender, zmq_receiver, frame_packet):
    
    zmq_sender.start()
    zmq_receiver.start()
    
    sender_thread = Thread(target=zmq_sender.send, args=(frame_packet,))
    receiver_thread = Thread(target=zmq_receiver.receive)
    
    receiver_thread.start()
    sleep(0.1)
    sender_thread.start()
    
    sender_thread.join()
    receiver_thread.join()
    
    zmq_sender.stop()
    zmq_receiver.stop()