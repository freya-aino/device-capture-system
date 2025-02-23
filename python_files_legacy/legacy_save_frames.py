import time
import argparse
import logging
import argparse

from datetime import datetime

from device_capture_system.deviceIO import load_all_devices_from_config
from device_capture_system.core import MultiInputStreamSender
from device_capture_system.datamodel import FramePreprocessing
from device_capture_system.fileIO import ImageSaver, VideoSaver

# ---------------------------------------------------------------------

AP = argparse.ArgumentParser()
AP.add_argument("--output_path", "-o", type=str, required=True, help="output path")
AP.add_argument("--save_type", "-t", type=str, required=True, help="save type", choices=["image", "video"])

AP.add_argument("--config", type=str, default="./configs/devices.json", help="path to device configuration file")
AP.add_argument("--host", type=str, default="127.0.0.1", help="host name or ip of the server")
AP.add_argument("--proxy_sub_port", type=int, default=10000, help="port for proxy subscriber")
AP.add_argument("--proxy_pub_port", type=int, default=10001, help="port for proxy publisher")
AP.add_argument("--zmq_proxy_queue_size", type=int, default=1000, help="zmq proxy queue size")

# image parameters
AP.add_argument("--num_images", type=int, default=100, help="number of images to save")
AP.add_argument("--image_file_extension", type=str, default="jpg", help="image file extension", choices=["jpg", "png"])
AP.add_argument("--jpg_quality", type=int, default=95, help="jpg quality")
AP.add_argument("--png_compression", type=int, default=3, help="png compression level")

# video parameters
AP.add_argument("--video_length", type=int, default=10, help="video length in seconds")
AP.add_argument("--video_codec", type=str, default="h264", help="video codec")
AP.add_argument("--inter_video_save_timer", type=int, default=3, help="time between saving videos")

AP.add_argument("-ll", "--logging_level", type=str, default="info", help="logging level", choices=["debug", "warning", "error"])
ARGS = AP.parse_args()

# ---------------------------------------------------------------------

logging.basicConfig(level=ARGS.logging_level.upper())
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------

FRAME_PREPROCESSINGS = {
    "Logitech-StreamCam.center": FramePreprocessing.ROTATE_90_CLOCKWISE,
    "Razer-Kiyo-1.left": FramePreprocessing.ROTATE_90_CLOCKWISE,
    "Razer-Kiyo-2.right": FramePreprocessing.ROTATE_90_COUNTERCLOCKWISE
}

# ---------------------------------------------------------------------

if __name__ == "__main__":
        
    cameras = load_all_devices_from_config("video", config_file=ARGS.config)
    
    logger.warning("FRAME PREPROCESSING IS SET MANUALLY IN THE CODE TO:\n" + ''.join([f'{cam.name} => {FRAME_PREPROCESSINGS.get(cam.name, None)}\n' for cam in cameras]))
    
    if ARGS.save_type == "video":
        saver = VideoSaver(
            cameras=cameras,
            proxy_pub_port=ARGS.proxy_pub_port,
            output_path=ARGS.output_path,
            video_length=ARGS.video_length,
            codec=ARGS.video_codec,
            host=ARGS.host
        )
    else:
        saver = ImageSaver(
            cameras=cameras,
            proxy_pub_port=ARGS.proxy_pub_port,
            output_path=ARGS.output_path,
            image_file_extension=ARGS.image_file_extension,
            jpg_quality=ARGS.jpg_quality,
            png_compression=ARGS.png_compression,
            host=ARGS.host
        )
    
    input_stream_sender = MultiInputStreamSender(
        devices=cameras,
        proxy_sub_port=ARGS.proxy_sub_port,
        proxy_pub_port=ARGS.proxy_pub_port,
        host=ARGS.host,
        zmq_proxy_queue_size=ARGS.zmq_proxy_queue_size,
        frame_preprocessings=FRAME_PREPROCESSINGS
    )
    
    try:
        
        saver.start()
        input_stream_sender.start_processes()
        
        if ARGS.save_type == "video":
            saver.save_video(video_name=f"{datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}")
            time.sleep(ARGS.inter_video_save_timer)
        else:
            saver.save_images(ARGS.num_images)
        
    except Exception as e:
        logger.error(e.with_traceback())
    finally:
        saver.stop()
        input_stream_sender.stop_processes()
