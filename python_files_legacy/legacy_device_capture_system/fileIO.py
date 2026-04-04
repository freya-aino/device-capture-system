import av
import os
import cv2
import numpy as np

from datetime import datetime
from PIL import Image
from time import sleep
from logging import getLogger
from multiprocessing import Pool
from typing import List

from .datamodel import VideoFile, ImageFile, CameraDevice, FramePacket
from .core import InputStreamReceiver

# ---------------------------------------------------------------------

class VideoSaver:
    
    def __init__(
        self, 
        cameras: List[CameraDevice], 
        proxy_pub_port: int, 
        output_path: str,
        video_length: int,
        codec: str = "h264",
        host: str = "127.0.0.1"):
        self.logger = getLogger(f"{self.__class__.__name__}")
        
        self.cameras = cameras
        self.stream_receiver = InputStreamReceiver(devices=cameras, proxy_pub_port=proxy_pub_port, host=host)
        
        assert len(np.unique([cam.name for cam in cameras])) == len(cameras), "All cameras must have unique names"
        
        # initialize video files
        self.video_files = [
            VideoFile(
                file_path=os.path.join(output_path, cam.name),
                file_name="placeholder", # set in VideoSaver.save_video
                file_extension="mp4",
                width=cam.height, # ! the cameras are all rotated by 90 degrees
                height=cam.width, # ! the cameras are all rotated by 90 degrees
                fps=cam.fps,
                seconds=video_length,
                codec=codec
            ) for cam in cameras]
        
        # create directories if not exist
        for video_file in self.video_files:
            if not os.path.exists(video_file.file_path):
                os.makedirs(video_file.file_path)
                self.logger.info(f"directory {video_file.file_path} created")
        
    def start(self):
        self.stream_receiver.start()
        
    def stop(self):
        self.stream_receiver.stop()
        
    def save_video(self, video_name: str, bad_frames_timeout: int = 25):
        
        output_files = []
        streams = []
        for video_file in self.video_files:
            output_file = av.open(file=os.path.join(video_file.file_path, f"{video_name}.{video_file.file_extension}"), mode="w")
            stream = output_file.add_stream(codec_name=video_file.codec, rate=video_file.fps)
            stream.width = video_file.width
            stream.height = video_file.height
            stream.pix_fmt = "yuv420p"
            
            output_files.append(output_file)
            streams.append(stream)
        
        frames_to_collect = video_file.fps * video_file.seconds
        collected_frames = 0
        timeout_counter = 0
        
        try:
            
            self.logger.info("saving video ...")
            
            while collected_frames < frames_to_collect:
                
                frames = self.stream_receiver.read()
                
                # check if frames is None, if so increment timeout counter and wait for 1 second
                if frames is None:
                    sleep(1)
                    timeout_counter += 1
                    self.logger.info(f"timeout while waiting for frames: {timeout_counter}/{bad_frames_timeout}")
                    assert timeout_counter < bad_frames_timeout, f"timeout while waiting for frames"
                    continue
                
                timeout_counter = 0
                collected_frames += 1
                
                for (i, cam) in enumerate(self.cameras):
                    
                    av_frame = av.VideoFrame.from_ndarray(frames[cam.device_id].frame, format="rgb24")
                    av_frame = av_frame.reformat(format="yuv420p")
                    
                    for packet in streams[i].encode(av_frame):
                        output_files[i].mux(packet)
                
                self.logger.info(f"writing {collected_frames}/{frames_to_collect} frames")
            
            # flush the encoder
            for i in range(len(self.cameras)):
                for packet in streams[i].encode():
                    output_files[i].mux(packet)
            
            self.logger.info(f"video saved !")
            
        except Exception as e:
            # self.logger.error(f"Error while saving video: {e}")
            raise e
        finally:
            self.logger.info("closing video files ...")
            for of in output_files:
                of.close()

class ImageSaver:
    
    def __init__(
        self, 
        cameras: List[CameraDevice], 
        proxy_pub_port: int,
        output_path: str,
        image_file_extension: str = "jpg",
        host: str = "127.0.0.1", 
        jpg_quality: int = 95,
        png_compression: int = 3,
        num_workers: int = 8):
        self.logger = getLogger(f"{self.__class__.__name__}")
        
        assert len(np.unique([cam.name for cam in cameras])) == len(cameras), "All cameras must have unique names"
        
        # set image parameters
        self.cameras = cameras
        self.image_files = [
            ImageFile(
                file_path=os.path.join(output_path, cam.name),
                file_name="placeholder", # set in ImageSaver.save_image
                file_extension=image_file_extension,
                jpg_quality=jpg_quality,
                png_compression=png_compression
            ) for cam in cameras]
        
        # set receiver
        self.stream_receiver = InputStreamReceiver(devices=cameras, proxy_pub_port=proxy_pub_port, host=host)
        
        # instantiate a multiprocessing worker pool placeholder for saving images
        self.pool = None
        self.num_workers = num_workers
        self.futures = []
        
        # create directories if not exist
        for image_file in self.image_files:
            if not os.path.exists(image_file.file_path):
                os.makedirs(image_file.file_path)
                self.logger.info(f"directory {image_file.file_path} created")
        
        
    def start(self):
        assert self.pool is None, "trying to start a process that has already started"
        self.stream_receiver.start()
        self.pool = Pool(self.num_workers)
        
    def stop(self):
        self.stream_receiver.stop()
        
        for result in self.futures:
            try:
                self.logger.info(f"Waiting for process {result} to finish ...")
                result.get(timeout=1)
            except Exception as e:
                self.logger.error(f"Error while waiting for process to finish: {e}")
        
        if self.pool is not None:
            self.pool.close()
        
        for worker in self.pool._pool:
            try:
                self.logger.info(f"Joining worker {worker} ...")
                worker.join(timeout=1)
            except Exception as e:
                self.logger.error(f"Error while joining worker: {e}, terminating ...")
                worker.terminate()
        
        self.pool.join()
        self.pool = None
        
    def check_futures(self):
        for result in self.futures:
            if result.ready():
                try:
                    result.get()
                except Exception as e:
                    self.logger.error(f"Error produced by future: {e}")
    
    @staticmethod
    def _save_image(frame: np.ndarray, image_file: ImageFile, image_name: str):
        
        image_uri = os.path.join(image_file.file_path, f"{image_name}.{image_file.file_extension}")
        image = Image.fromarray(frame)
        
        if image_file.file_extension == "jpg":
            image.save(image_uri, quality=image_file.jpg_quality)
        elif image_file.file_extension == "png":
            image.save(image_uri, compress_level=image_file.png_compression)
        
    def save_image(self, image_name: str) -> bool:
        
        frames = self.stream_receiver.read()
        
        # check if frames is None, if so increment timeout counter and wait for 1 second
        if frames is None:
            return False
        
        # clean up results
        self.check_futures() # check for errors
        self.futures = [r for r in self.futures if not r.ready()]
        self.logger.debug(f"not-ready process results in queue: {len(self.futures)}")
        
        # save images
        self.logger.info(f"saving images {image_name} ...")
        for (i, camera) in enumerate(self.cameras):
            cam_id = camera.device_id
            result = self.pool.apply_async(ImageSaver._save_image, (frames[cam_id].frame, self.image_files[i], image_name))
            self.futures.append(result)
        
        return True
    
    def save_images(self, number_of_images: int, bad_frames_timeout: int = 25):
        
        saved_images = 0
        timeout_counter = 0
        while saved_images < number_of_images:
            
            ok = self.save_image(f"{datetime.now().strftime('%Y-%m-%d_%H-%M-%S-%f')}")
            if not ok:
                sleep(1)
                timeout_counter += 1
                self.logger.info(f"timeout while waiting for frames: {timeout_counter}/{bad_frames_timeout}")
                assert timeout_counter < bad_frames_timeout, f"timeout while waiting for frames"
                continue
            
            timeout_counter = 0
            saved_images += 1