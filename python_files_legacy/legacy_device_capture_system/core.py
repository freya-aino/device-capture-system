import time
import numpy as np

from typing import Dict, Callable, List
from time import sleep
from logging import getLogger
from concurrent.futures import ThreadPoolExecutor
from multiprocessing import Process, Event
from multiprocessing import TimeoutError as ProcessTimeoutError

from .datamodel import PeripheryDevice, CameraDevice, AudioDevice, FramePreprocessing
from .deviceIO import CameraDeviceReader, AudioDeviceReader
from .zmqIO import ZMQSender, ZMQReceiver, ZMQProxy

# ------------- SINGLE STREAM CLASSES -------------

class InputStreamSender:
    def __init__(
        self, 
        device: PeripheryDevice, 
        proxy_sub_port: int, 
        host: str = "127.0.0.1", 
        zmq_sender_queue_size: int = 10,
        frame_preprocessing: FramePreprocessing = None, 
        invalid_frame_timeout: float = 1.):
        self.logger = getLogger(f"{self.__class__.__name__}:{device.name}")
        
        self.device = device
        self.host = host
        self.proxy_port = proxy_sub_port
        self.frame_preprocessing = frame_preprocessing
        self.zmq_sender_queue_size = zmq_sender_queue_size
        
        # timeouts
        self.invalid_frame_timeout = invalid_frame_timeout
        
        # for multiprocessing
        self.stop_event = Event()
        self.process = None
    
    def is_active(self): return self.process is not None
    
    def start_process(self):
        
        assert not self.is_active(), "trying to start a process that has already started ..."
        
        self.stop_event.clear()
        self.process = Process(target=self._run)
        self.process.start()
        
    def stop_process(self, timeout=1):
        
        self.stop_event.set()
        
        if self.process is None: return
        
        try:
            self.process.join(timeout=timeout)
        except ProcessTimeoutError:
            self.logger.warning("process join timeout, terminating ...")
            self.process.terminate()
            self.process.join()
        
        self.process = None
        
    def _run(self):
        
        # crteate zmq sender
        zmq_sender = ZMQSender(host=self.host, port=self.proxy_port, q_size=self.zmq_sender_queue_size, name=self.device.name)
        
        # create device reader
        if isinstance(self.device, CameraDevice):
            device_reader = CameraDeviceReader(self.device)
        elif isinstance(self.device, AudioDevice):
            device_reader = AudioDeviceReader(self.device)
        else:
            raise ValueError("device type not supported")
        
        # set frame preprocessing
        if self.frame_preprocessing == FramePreprocessing.ROTATE_180:
            preprocess = lambda frame: np.rot90(frame, 2)
        elif self.frame_preprocessing == FramePreprocessing.ROTATE_90_CLOCKWISE:
            preprocess = lambda frame: np.rot90(frame, 1)
        elif self.frame_preprocessing == FramePreprocessing.ROTATE_90_COUNTERCLOCKWISE:
            preprocess = lambda frame: np.rot90(frame, 3)
        else:
            preprocess = lambda frame: frame
        
        # start continuous read frame -> preprocess -> send frame
        try:
            device_reader.start()
            zmq_sender.start()
            
            while not self.stop_event.is_set():
                
                dt = time.perf_counter()
                
                # read frame
                frame_packet = device_reader.read()
                if frame_packet is None:
                    sleep(self.invalid_frame_timeout)
                    continue
                
                # preprocess frame
                frame_packet.frame = preprocess(frame_packet.frame)
                
                # send frame
                zmq_sender.send(frame_packet)
                
                send_time = (time.perf_counter() - dt)
                if send_time > 0:
                    self.logger.debug(f"frame read and sent with fps {1 / send_time}")
                
        except Exception as e:
            raise e
        finally:
            zmq_sender.stop()
            device_reader.stop()

class InputStreamReceiver:
    
    def __init__(self, devices: List[PeripheryDevice], proxy_pub_port: int, host: str = "127.0.0.1", zmq_receiver_queue_size: int = 10):
        self.logger = getLogger(f"{self.__class__.__name__}" )
        self.devices = devices
        self.zmq_receiver = ZMQReceiver(host=host, port=proxy_pub_port, q_size=zmq_receiver_queue_size)
        self.running = False
    
    def start(self):
        self.running = True
        self.zmq_receiver.start()
        
    def stop(self):
        self.running = False
        self.zmq_receiver.stop()
        
    def read(self, read_attemps: int = 10):
        output = {}
        
        while len(output) < len(self.devices) and self.running:
            
            if read_attemps <= 0:
                return None
            
            frame_packet = self.zmq_receiver.receive()
            
            if frame_packet is None:
                read_attemps -= 1
                continue
            if frame_packet.device.device_id in output:
                read_attemps -= 1
            
            output[frame_packet.device.device_id] = frame_packet
        
        return output

# ------------- MULTI STREAM CLASSES -------------

class MultiInputStreamSender:
    
    def __init__(
        self, 
        devices: List[PeripheryDevice], 
        proxy_sub_port: int, 
        proxy_pub_port: int, 
        host: str = "127.0.0.1", 
        zmq_proxy_queue_size: int = 10,
        zmq_sender_queue_size: int = 10,
        frame_preprocessings: Dict[str, FramePreprocessing] = []):
        self.logger = getLogger(self.__class__.__name__)
        
        self.input_sender = [
            InputStreamSender(
                device = device, 
                proxy_sub_port = proxy_sub_port, 
                host = host,
                zmq_sender_queue_size = zmq_sender_queue_size,
                frame_preprocessing = frame_preprocessings.get(device.name, None)
            ) 
            for device in devices
        ]
        self.zmq_proxy = ZMQProxy(host, sub_port=proxy_sub_port, pub_port=proxy_pub_port, queue_size=zmq_proxy_queue_size)
        
        self.logger.info(f"multi input stream sender with {len(self.input_sender)} senders")
        
    def start_processes(self):
        
        self.zmq_proxy.start_process()
        
        # start all input senders in parallel
        thread_pool_executor = ThreadPoolExecutor(max_workers=len(self.input_sender))
        
        futures = [thread_pool_executor.submit(sub.start_process) for sub in self.input_sender]
        
        # wait for all senders to start
        results = [future.result(timeout=5) for future in futures]
        return results
        
    def stop_processes(self, timeout: float = 1):
        
        for sub in self.input_sender:
            sub.stop_process(timeout=timeout)
        
        self.zmq_proxy.stop_process()
        
        

# class MultiInputStreamReceiver:
    
#     def __init__(self, sender_port: int, receiver_port, host: str = "127.0.0.1"):
        
#         self.logger = getLogger(self.__class__.__name__)
#         self.capture_receiver = InputStreamReceiver(port, host)
#         self.proxy = ZMQProxy(host, )
        
#         self.executor = ThreadPoolExecutor(max_workers=len(self.capture_receiver))
        
        
#     def start(self):
#         for rec in self.capture_receiver:
#             rec.start()
        
#     def stop(self):
#         for rec in self.capture_receiver:
#             rec.stop()
        
#     def read(self):
#         futures = [self.executor.submit(rec.read) for rec in self.capture_receiver]
#         frames = [future.result() for future in futures]
#         if any([frame is None for frame in frames]):
#             return None
#         return frames

#     def read(self, block: bool = False, timeout: float = 1, synchronous_read: bool = False) -> Union[List[FramePacket], None]:
        
#         # read from all camera subseiber
#         frame_packets = [self.capture_subscribers[k].read(block=block, timeout=timeout) for k in self.capture_subscribers]
        
#         self.logger.debug(f"valid frame packets: {[f is not None for f in frame_packets]}")
        
#         # return if any packets are None
#         for packet in frame_packets:
#             if packet is None:
#                 return None
        
#         if not synchronous_read:
#             return frame_packets
        
#         # syncronize frames
#         # compute last frames datetime on first read
#         if self.last_frames_datetime is None:
#             self.last_frames_datetime = [*map(lambda a: a.end_read_dt, frame_packets)]
#             return frame_packets
        
#         # read frames until all datetimes line up as accuratl as possible
#         most_recent_last_frame_dt = max(self.last_frames_datetime)
#         for i in range(len(frame_packets)):
#             while frame_packets[i].end_read_dt < most_recent_last_frame_dt:
#                 new_frame = self.capture_subscribers[frame_packets[i].device.uuid].read(block=True, timeout=timeout)
                
#                 if new_frame is None:
#                     self.logger.warn("failed to read next frame")
#                     sleep(0.1)
                
#                 self.last_frames_datetime[i] = frame_packets[i].end_read_dt
#                 frame_packets[i] = new_frame
        
#         return frame_packets
    
#     def empty_queues(self):
#         self.logger.info("emptying capture queues ...")
        
#         # set empty queue event
#         for capture_subscriber in self.capture_subscribers.values():
#             capture_subscriber.queue_empty_event.set()
        
#         for capture_subscriber in self.capture_subscribers.values():
#             while capture_subscriber.output_queue.qsize() > 0:
                
#                 try:
#                     capture_subscriber.output_queue.get(timeout=1)
#                 except ValueError:
#                     self.logger.warning("failed to empty queue")
#                     break
        
#         # clear empty queue event
#         for capture_subscriber in self.capture_subscribers.values():
#             capture_subscriber.queue_empty_event.clear()
        
#         self.logger.info("queues emptied !")
