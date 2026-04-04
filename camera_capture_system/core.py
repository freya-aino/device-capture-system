from time import sleep
from logging import getLogger
from typing import List, Callable
from json import load
from traceback import format_exc
from multiprocessing import Process, Event, Queue
from datetime import datetime, timedelta
import queue

from .datamodel import Camera, CameraFramePacket
from .cameraIO import CameraInputReader
from .zmqIO import ZMQPublisher, ZMQSubscriber

#----------------------------------------

logger = getLogger(__name__)

#----------------------------------------

def load_all_cameras_from_config(config_path: str) -> List[Camera]:
    logger.debug(f"Loading cameras from {config_path} ...")
    with open(config_path, "r") as f:
        cameras = load(f)
    return [Camera(**cameras[cam_uuid], uuid=cam_uuid) for cam_uuid in cameras]

class CaptureSubscriber:
    """
        Subscribes to a ZMQ socket and returns the data.
    """
    
    def __init__(self, camera: Camera, q_size: int, host: str = "127.0.0.1"):
        
        self.camera = camera
        self. host = host
        
        self.stop_event = Event()
        self.output_queue = Queue(maxsize=q_size)
        self.q_size = q_size
        self.process = None
        
    def start_process(self):
        if self.process is not None:
            logger.warning(f"{self.camera.uuid} :: trying to start a process that has already started")
            return
        
        logger.info(f"{self.camera.uuid} :: starting capture subscriber process ...")
        self.stop_event.clear()
        process = Process(target=self._start)
        process.start()
        self.process = process
        
    def stop_process(self, terminate: bool):
        if self.process is None:
            logger.warning(f"{self.camera.uuid} :: trying to stop a process that has not started")
            return
        
        logger.info(f"{self.camera.uuid} :: stopping capture subscriber process and clear queue ...")
        self.output_queue = Queue(maxsize=self.q_size)
        self.stop_event.set()
        self.process.join(timeout=1)
        if terminate:
            self.process.terminate()
        self.process = None
        
    def _start(self):
        
        logger.info(f"{self.camera.uuid} :: starting capture subscriber")
        
        try:
            
            zmq_subscriber = ZMQSubscriber(self.host, self.camera.publishing_port)
            
            while self.stop_event is None or not self.stop_event.is_set():
                
                assert zmq_subscriber.is_ok(), f"{self.camera.uuid} :: zmq subscriber is not ok"
                
                # read frame from capture and put in queue
                try:
                    frame_packet = zmq_subscriber.recieve()
                    self.output_queue.put(frame_packet, block=False)
                except queue.Full:
                    # logger.warning(f"{self.camera.uuid} :: capture subscriber queue is full, dropping frame")
                    continue
                
        except KeyboardInterrupt:
            logger.info("KeyboardInterrupt ...")
        except:
            raise
        finally:
            zmq_subscriber.close()
            
    def read(self, block: bool = False, timeout: float = 1):
        try:
            return self.output_queue.get(block=block, timeout=timeout)
        except queue.Empty:
            return None
        except:
            raise

class MultiCaptureSubscriber:
    """
        Subsribes to a list of ZMQ sockets as processes and returns the data.
        
        ! calling stop() at termination is the responsibility of the caller !
    """
    
    def __init__(self, cameras: List[Camera], q_size: int, host: str = "127.0.0.1"):
        self.capture_subscribers = {cam.uuid: CaptureSubscriber(cam, q_size, host) for cam in cameras}
        
        self.last_frames_datetime = {cam.uuid: None for cam in cameras}
        
    def stop(self, terminate : bool = True):
        # stop all capture subscribers processes and wait for cleanup
        for capture_subscriber in self.capture_subscribers.values():
            capture_subscriber.stop_process(terminate=terminate)
        logger.info("multi cam capture subscriber: stopped")
        
    def start(self):
        # start reading from all cameras
        for capture_subscriber in self.capture_subscribers.values():
            capture_subscriber.start_process()
        
    def read(self, block: bool = False, timeout: float = 1, synchronous_read: bool = False) -> List[CameraFramePacket]:
        
        # read from all camera subseiber
        frame_packets = [self.capture_subscribers[k].read(block=block, timeout=timeout) for k in self.capture_subscribers]
        
        if any([frame_packet is None for frame_packet in frame_packets]):
            logger.warn("some or all captures returned None ...")
            return None
        
        if not synchronous_read:
            return frame_packets
        
        # syncronize frames
        # compute last frames datetime on first read
        if any([self.last_frames_datetime[k] is None for k in self.last_frames_datetime]):
            self.last_frames_datetime = dict(zip(self.capture_subscribers, map(lambda a: a.end_read_dt, frame_packets)))
            return frame_packets
        
        # determine which queue needs to skip frames to get to the latest syncronized frame
        for (i, k) in enumerate(self.capture_subscribers):
            
            condition = any([frame_packets[i].end_read_dt < self.last_frames_datetime[kk] for kk in self.capture_subscribers if kk != k])
            
            while condition:
                
                # update last frames datetime
                self.last_frames_datetime[k] = frame_packets[i].end_read_dt
                
                frame_packets[i] = self.capture_subscribers[k].read(block=True, timeout=timeout)
                condition = any([frame_packets[i].end_read_dt < self.last_frames_datetime[kk] for kk in self.capture_subscribers if kk != k])
        
        # update last frames datetime
        for (i, k) in enumerate(self.capture_subscribers):
            self.last_frames_datetime[k] = frame_packets[i].end_read_dt
        
        return frame_packets

class CapturePublisher:
    """
        Publishes data from a single camera to a ZMQ socket.
    """
    
    def __init__(self, camera: Camera, host: int = "127.0.0.1", frame_transform: str = None):
        
        self.camera = camera
        self.host = host
        self.frame_transform = frame_transform
        
        # for multiprocessing
        self.stop_event = Event()
        self.process = None
        
    def start_process(self):
        if self.process is not None:
            logger.warning(f"{self.camera.uuid} :: trying to start a process that has already started")
            return
        
        logger.info(f"{self.camera.uuid} :: starting capture publisher process ...")
        self.stop_event.clear()
        process = Process(target=self._start)
        process.start()
        self.process = process
        
    def stop_process(self, terminate=False):
        if self.process is None:
            logger.warning(f"{self.camera.uuid} :: trying to stop a process that has not started")
            return
        
        logger.info(f"{self.camera.uuid} :: stopping capture publisher process ...")
        self.stop_event.set()
        if terminate:
            self.process.terminate()
        self.process.join()
        self.process = None
        
    def _start(self):
        
        logger.info(f"{self.camera.uuid} :: starting capture publisher")
        
        zmq_publisher = ZMQPublisher(self.host, self.camera.publishing_port)
        capture = CameraInputReader(self.camera, frame_transform=self.frame_transform)
        
        try:
            
            
            while self.stop_event is None or not self.stop_event.is_set():
                
                assert zmq_publisher.is_ok(), f"{self.camera.uuid} :: zmq publisher is not ok"
                
                # read frame from capture
                frame_packet = capture.read()
                
                if frame_packet is None:
                    logger.warning(f"{self.camera.uuid} :: capture publisher failed to read frame")
                    continue
                
                # publish frame
                zmq_publisher.publish(frame_packet)
                
        except KeyboardInterrupt:
            logger.info("KeyboardInterrupt ...")
        except:
            raise
        finally:
            # close zmq publisher and capture from inside the process
            zmq_publisher.close()
            capture.close()

class MultiCapturePublisher:
    """
        Publishes multiple cameras data to individual ZMQ sockets.
        
        ! if background is set, the calling process is responsible for calling stop() at termination !
    """
    
    def __init__(self, cameras: List[Camera], host: str = "127.0.0.1", frame_transforms: dict[str, str] = {}):
        
        assert len(cameras) > 0, "no cameras provided"
        
        self.capture_publishers = {cam.uuid: CapturePublisher(cam, host, frame_transforms.get(cam.uuid, None)) for cam in cameras}
        
    def stop(self, terminate=False):
        # stop all capture publishers processes and wait for cleanup (unless terminate is set, then terminate immediately)
        for capture_publisher in self.capture_publishers.values():
            capture_publisher.stop_process(terminate=terminate)
        logger.info("multi cam capture publisher: stopped")
        
    def start(self, background: bool = True):
        
        logger.info("starting multi cam capture publisher ...")
        try:
            # start all capture publishers processes
            for capture_publisher in self.capture_publishers.values():
                capture_publisher.start_process()
        except:
            self.stop(terminate=True)
            raise
        logger.info("multi cam capture publisher started")
        
        if background:
            return
        
        try:
            while True:
                
                # check if any of the processes have stopped and restart if desired
                for capture_publisher in self.capture_publishers.values():
                    
                    if capture_publisher.process is None:
                        raise Exception(f"capture publisher for {capture_publisher.camera.uuid} has not started")
                    
                    if not capture_publisher.process.is_alive():
                        # TODO: self.restart(k)
                        raise Exception(f"capture publisher for {capture_publisher.camera.uuid} has stopped")
                
                sleep(0.5)
                
        except KeyboardInterrupt:
            logger.info("KeyboardInterrupt ...")
        except:
            raise
        finally:
            self.stop()
        
    def restart(self, cam_uuid):
        logger.info(f"restarting capture publisher for {self.cameras[cam_uuid].uuid} ...")
        self.capture_publishers[cam_uuid].stop_process()
        self.capture_publishers[cam_uuid].start_process()