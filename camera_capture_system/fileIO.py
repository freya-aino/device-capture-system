import os
import cv2

from time import sleep
from traceback import format_exc
from datetime import datetime
from logging import getLogger
from concurrent.futures import ThreadPoolExecutor
from multiprocessing import Queue, Process, Pool
from typing import List

from .datamodel import CameraFramePacket, ImageParameters, VideoParameters, Camera
from .core import MultiCaptureSubscriber

# ------------------- Logging ------------------- #

logger = getLogger(__name__)

# ------------------- Helpers ------------------- #

def create_camera_save_directories(cameras: List[Camera], save_path: str):
    
    # check save path exists
    assert os.path.exists(save_path), f"save path {save_path} does not exist"
    
    # create cam directory and create if necessary
    for cam in cameras:
        save_path_ = os.path.join(save_path, cam.uuid)
        if not os.path.exists(save_path_):
            os.makedirs(save_path_)
            logger.info(f"directory {save_path_} created")

def save_videos_(frames_packets: list, video_name: str, video_params: VideoParameters):
    
    video_writers = []
    
    try:
        first_frame_packets = frames_packets[0]
        
        # extract cam uuids
        cam_uuids = [fp.camera.uuid for fp in first_frame_packets]
        video_uris = [os.path.join(video_params.save_path, cam_uuid, f"{video_name}.{video_params.output_format}") for cam_uuid in cam_uuids]
        first_frames = [fp.camera_frame for fp in first_frame_packets]
        
        video_writers = [
            cv2.VideoWriter(
                filename = video_uris[i],
                fourcc=cv2.VideoWriter_fourcc(*video_params.codec),
                fps=video_params.fps, frameSize=(first_frames[i].shape[1], first_frames[i].shape[0])
            ) for i in range(len(first_frame_packets))]
        
        
        logger.info(f"start saving video {video_name}")
        
        # write frames to video
        for frame_packets in frames_packets:
            for i in range(len(frame_packets)):
                video_writers[i].write(frame_packets[i].camera_frame)
        
        # release video writer
        for vr in video_writers:
            vr.release()
            
        logger.info(f"done saving video {video_name} ...")
    
    except:
        logger.error(format_exc())
        raise
    finally:
        for vr in video_writers:
            if vr.isOpened():
                vr.release()
    
def save_image_(frame_packet: CameraFramePacket, image_uri: str, image_params: ImageParameters):
    try:
        if image_params.output_format == "jpg":
            cv2.imwrite(image_uri, frame_packet.camera_frame, [int(cv2.IMWRITE_JPEG_QUALITY), image_params.jpg_quality])
        elif image_params.output_format == "png":
            cv2.imwrite(image_uri, frame_packet.camera_frame, [int(cv2.IMWRITE_PNG_COMPRESSION), image_params.png_compression])
    except Exception as e:
        logger.error(f"Error while saving image: {e}")

# ------------------- Functionality ------------------- #

class CaptureVideoSaver(MultiCaptureSubscriber):
    
    def __init__(self, cameras: List[Camera], video_params: VideoParameters, host: str = "127.0.0.1"):
        
        self.video_params = video_params
        
        create_camera_save_directories(cameras=cameras, save_path=video_params.save_path)
        
        self.frames_per_video = video_params.fps * video_params.seconds
        self.save_video_processes = None
        
        super().__init__(cameras=cameras, host=host, q_size=self.frames_per_video)
        
    def start(self):
        
        # check if already started
        if self.save_video_processes is not None:
            logger.warning("trying to start a process that has already started")
            return
        
        logger.info("starting capture video saver ...")
        self.save_video_processes = []
        super().start()
        
    def stop(self, terminate: bool = True):
        
        # check if already stopped
        if self.save_video_processes is None:
            logger.warning("trying to stop a process that has already stopped")
            return
        
        for process in self.save_video_processes:
            try:
                logger.info("trying to stop capture video saver ...")
                process.join(timeout=1)
            except TimeoutError:
                logger.warn("TimeoutError while waiting for process to finish, terminating")
                process.terminate()
            except Exception as e:
                logger.error(f"Error while waiting for process to finish: {e}")
                process.terminate()
            
            process.close()
        
        self.save_video_processes = None
        
        super().stop(terminate=terminate)
        logger.info("all save video processes stopped")
    
    def save_video(self) -> bool:
        
        try:
            
            # clean up processes
            self.save_video_processes = [process for process in self.save_video_processes if process.is_alive()]
            
            
            logger.info(f"reading frames ...")
            video_name = datetime.now().isoformat().replace(":", "-")
            frames_packets = []
            captured_frames = 0
            
            while captured_frames < self.frames_per_video:
                
                frame_packets = self.read(block=True, timeout=1, synchronous_read=True)
                
                if frame_packets is None:
                    continue
                
                frames_packets.append(frame_packets)
                captured_frames += 1
                
                q_sizes = [cs.output_queue.qsize() for cs in self.capture_subscribers.values()]
                logger.info(f"captured frames: {captured_frames}/{self.frames_per_video} --- queue sizes: {q_sizes} --- {[dt.second for dt in self.last_frames_datetime.values()]}")
            
            # after collecting enough frames, prepare processes for saving in the background
            save_video_process = Process(
                target=save_videos_, 
                args=(
                    frames_packets,
                    video_name, 
                    self.video_params
                ),
                daemon=True)
            
            # append process reference
            self.save_video_processes.append(save_video_process)
            
            # start processes
            save_video_process.start()
            
        except:
            logger.error(format_exc())
            self.stop()
            raise
        
        return True

class CaptureImageSaver(MultiCaptureSubscriber):
    
    def __init__(self, cameras: List[Camera], image_params: ImageParameters, host: str = "127.0.0.1", num_workers: int = 8):
        
        create_camera_save_directories(cameras=cameras, save_path=image_params.save_path)
        
        # initialize Capture subscribers
        super().__init__(cameras=cameras, host=host, q_size=1)
        
        self.image_params = image_params
        
        # instantiate a multiprocessing worker pool placeholder for saving images
        self.pool = None
        self.num_workers = num_workers
        self.results = []
    
    def start(self):
        # check if already started
        if self.pool is not None:
            logger.warning("trying to start a process that has already started")
            return
        
        logger.info("starting capture image saver ...")
        self.pool = Pool(self.num_workers)
        super().start()
    
    def stop(self, terminate: bool = True):
        # check if already stopped
        if self.pool is None:
            logger.warning("trying to stop a process that has already stopped")
            return
        
        logger.info("stopping capture image saver and await current process results ...")
        
        # wait for all results or terminate
        for result in self.results:
            try:
                logger.info(f"waiting for process {result} to finish ...")
                result.get(timeout=1)
            except TimeoutError:
                logger.warn("TimeoutError while waiting for process to finish")
            except Exception as e:
                logger.error(f"Error while waiting for process to finish: {e}")
        
        self.pool.close()
        
        for worker in self.pool._pool:
            try:
                logger.info(f"joining worker {worker} ...")
                worker.join(timeout=1)
            except TimeoutError:
                logger.warn("TimeoutError while joining worker, terminating ...")
                worker.terminate()
            except Exception as e:
                logger.error(f"Error while joining worker: {e}, terminating ...")
                worker.terminate()
        
        if terminate:
            self.pool.terminate()
        self.pool = None
        super().stop(terminate=terminate)
        
    def save_image(self, visualize: bool = False) -> bool:
        
        # create datetime for image name
        image_name = datetime.now().isoformat().replace(":", "-")
        
        # read frames
        frame_packets = self.read(block=True, timeout=1)
        if any([frame_packet is None for frame_packet in frame_packets]):
            logger.warn("some or all captures returned None, skipping ...")
            return False
        
        # clean up results
        self.results = [result for result in self.results if not result.ready()]
        logger.info(f"not-ready process results in queue: {len(self.results)}")
        
        try:
            
            # save frames
            for frame_packet in frame_packets:
                
                # start save image process
                image_uri = os.path.join(self.image_params.save_path, frame_packet.camera.uuid, f"{image_name}.{self.image_params.output_format}")
                
                logger.info(f"saving images in {image_uri} ...")
                
                result = self.pool.apply_async(save_image_, (frame_packet, image_uri, self.image_params))
                self.results.append(result)
                
                # visualize
                if visualize:
                    cv2.imshow(frame_packet.camera.uuid, frame_packet.camera_frame)
                    cv2.waitKey(1)
            
            # # Check for errors
            # for result in self.results:
            #     try:
            #         result.get(timeout=1)  # This will raise an exception if the function failed
            #     except Exception as e:
            #         logger.error(f"Error in save_image function: {e}")
            
        except:
            self.stop()
            cv2.destroyAllWindows()
            raise
        
        return True