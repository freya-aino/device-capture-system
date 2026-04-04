import zmq
import importlib
import json

from multiprocessing import Process, Pipe
from multiprocessing import TimeoutError as ProcessTimeoutError
from datetime import datetime
from numpy import frombuffer
from logging import getLogger

from device_capture_system import datamodel



class ZMQProxy():
    def __init__(self, host: str, sub_port: int, pub_port: int, queue_size: int = 10):
        self.logger = getLogger(f"{self.__class__.__name__}@{host}:{sub_port}->{pub_port}")
        
        self.queue_size = queue_size
        self.host = host
        self.sub_port = sub_port
        self.pub_port = pub_port
        self.process = None
        
    def is_active(self):
        return self.process is not None and self.process.is_alive()
    
    def start_process(self):
        self.logger.info("starting ...")
        
        assert not self.is_active(), "trying to start proxy that has already started, restarting ..."
        
        parent_pipe, child_pipe = Pipe()
        self.process = Process(target=self._run, args=(child_pipe,))
        self.process.start()
        
        self.logger.info("started !")
        
    def stop_process(self):
        self.logger.info("stopping ...")
        
        if self.process is not None:
            self.process.terminate()
            self.process.join()
            # try:
            #     self.process.join()
            # except ProcessTimeoutError:
            #     self.logger.warning("process join timeout, terminating ...")
            #     self.process.terminate()
        self.process = None
        
        self.logger.info("stopped !")
    
    def _run(self, pipe):
        
        context = zmq.Context()
        xsub_socket = context.socket(zmq.XSUB)
        xpub_socket = context.socket(zmq.XPUB)
        
        xsub_socket.setsockopt(zmq.RCVHWM, self.queue_size)
        xpub_socket.setsockopt(zmq.SNDHWM, self.queue_size)
        
        try:
            xsub_socket.bind(f"tcp://{self.host}:{self.sub_port}")
            
            xpub_socket.bind(f"tcp://{self.host}:{self.pub_port}")
            
            zmq.proxy(xsub_socket, xpub_socket)
            pipe.send(None) # signal to parent that the proxy has stopped
        except Exception as e:
            raise e
        finally:
            xsub_socket.close()
            xpub_socket.close()
            context.term()

class ZMQSender():
    
    def __init__(self, host: str, port: int, q_size: int = 10):
        
        self.logger = getLogger(f"{self.__class__.__name__}@{host}:{port}")
        
        self.host = host
        self.port = port
        self.q_size = q_size
        
        self.context = None
        self.socket = None
    
    def is_active(self):
        return self.context is not None
    
    def start(self):
        self.logger.info("starting ...")
        
        assert not self.is_active(), "trying to start a sender that has already started"
        
        self.context = zmq.Context()
        self.socket = self.context.socket(zmq.PUB)
        self.socket.setsockopt(zmq.SNDHWM, self.q_size)
        self.socket.connect(f"tcp://{self.host}:{self.port}")
        
        self.logger.info("started !")
    
    def stop(self):
        self.logger.info("stoping ...")
        if self.socket is not None:
            self.socket.close()
        if self.context is not None:
            self.context.term()
        self.context = None
        self.socket = None
        self.logger.info("stoped !")
    
    def send(self, packet: datamodel.FramePacket):
        
        if not self.is_active():
            self.logger.warning("trying to send data without starting the sender !")
            return
        
        self.logger.debug("sending data ...")
        
        packet_dump = packet.dump()
        frame = packet_dump["frame"]
        data = packet_dump["data"]
        
        try:
            self.socket.send_json(data, zmq.SNDMORE | zmq.NOBLOCK)
            self.socket.send(frame, flags=zmq.NOBLOCK, copy=False, track=False)
            self.logger.debug("data sent ...")
        except zmq.error.Again:
            self.logger.warning("could not send data")

class ZMQReceiver():
    
    def __init__(self, host: str, port: int, q_size: int = 10, receive_wait_time_ms: int = 1000):
        
        self.logger = getLogger(f"{self.__class__.__name__}@{host}:{port}")
        
        self.host = host
        self.port = port
        self.q_size = q_size
        self.receive_wait_time_ms = receive_wait_time_ms
        
        self.context = None
        self.socket = None
        
    def is_active(self):
        return self.context is not None
    
    def start(self):
        self.logger.info("starting ...")
        
        assert not self.is_active(), "trying to start a receiver that has already started"
        
        self.context = zmq.Context()
        self.socket = self.context.socket(zmq.SUB)
        self.socket.setsockopt_string(zmq.SUBSCRIBE, "")
        self.socket.setsockopt(zmq.RCVTIMEO, self.receive_wait_time_ms)
        self.socket.setsockopt(zmq.RCVHWM, self.q_size)
        self.socket.connect(f"tcp://{self.host}:{self.port}")
        
        self.logger.info("started !")
    
    def stop(self):
        self.logger.info("stopping ...")
        
        if self.socket is not None:
            self.socket.close()
        if self.context is not None:
            self.context.term()
        self.context = None
        self.socket = None
        
        self.logger.info("stopped !")
    
    def receive(self) -> datamodel.FramePacket:
        
        if not self.is_active():
            self.logger.warning("trying to receive data without starting the receiver !")
            return None
        
        try:
            data = self.socket.recv_json()
            frame = self.socket.recv(copy=False, track=False)
        except zmq.error.Again as e:
            self.logger.warning(f"could not receive data: {e}")
            return None
        except zmq.error.ZMQError as e:
            self.logger.warning(f"ZMQ error: {e}")
            return None
        
        self.logger.debug("data received ...")
        
        # format frame
        frame = frombuffer(frame, dtype=data["frame"]["dtype"])
        frame = frame.reshape(data["frame"]["shape"])
        
        # format device
        device_class = getattr(datamodel, data["device"]["type"])
        device = device_class(**data["device"]["parameters"])
        
        # extract timestamp
        start_read_dt = datetime.fromtimestamp(data["start_read_timestamp"])
        end_read_dt = datetime.fromtimestamp(data["end_read_timestamp"])
        
        # debug fps
        total_time = datetime.now().timestamp() - data['start_read_timestamp']
        if total_time > 0:
            self.logger.debug(f"fps from read to receive: {1 / total_time}")
        
        return datamodel.FramePacket(
            device=device,
            frame=frame,
            start_read_dt=start_read_dt,
            end_read_dt=end_read_dt
        )
