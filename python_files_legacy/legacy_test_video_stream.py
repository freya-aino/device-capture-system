import cv2
import time
import argparse

from logging import getLogger, basicConfig
from traceback import format_exc

from device_capture_system.core import MultiInputStreamSender, InputStreamReceiver
from device_capture_system.deviceIO import load_all_devices_from_config
from device_capture_system.datamodel import FramePreprocessing

# ---------------------------------------------------------------------

AP = argparse.ArgumentParser()
AP.add_argument("--config", type=str, default="./configs/devices.json", help="path to device config file")
AP.add_argument("--proxy_sub_port", type=int, default=10000, help="port for proxy subscriber")
AP.add_argument("--proxy_pub_port", type=int, default=10001, help="port for proxy publisher")
AP.add_argument("--host", type=str, default="127.0.0.1", help="host name or ip of the server")
AP.add_argument("-ll", "--logging_level", type=str, default="info", help="logging level", choices=["debug", "warning", "error"])
AP.add_argument("--num_frames", type=int, default=300, help="number of frames to collect")

ARGS = AP.parse_args()

# ---------------------------------------------------------------------

basicConfig(level=ARGS.logging_level.upper())
logger = getLogger(__name__)

# ---------------------------------------------------------------------

if __name__ == "__main__":
    
    cameras = load_all_devices_from_config("video", config_file=ARGS.config)
    
    for cam in cameras:
        print(cam)
    
    multi_sender = MultiInputStreamSender(
        devices = cameras,
        proxy_sub_port=ARGS.proxy_sub_port,
        proxy_pub_port=ARGS.proxy_pub_port,
        host = ARGS.host,
        frame_preprocessings=[
            FramePreprocessing.ROTATE_90_CLOCKWISE,
            FramePreprocessing.ROTATE_90_CLOCKWISE,
            FramePreprocessing.ROTATE_90_COUNTERCLOCKWISE
        ]
    )
    
    receiver = InputStreamReceiver(
        devices = cameras,
        proxy_pub_port = ARGS.proxy_pub_port,
        host = ARGS.host
    )
    
    try:
        
        receiver.start()
        multi_sender.start_processes()
        
        time_taken = 0
        collected_frames = 0
        frames_to_collect = ARGS.num_frames
        while collected_frames < frames_to_collect:
            dt = time.time()
            frames = receiver.read()
            if frames is None:
                continue
            time_taken += time.time() - dt
            collected_frames += 1
            
            for k in frames:
                cv2.imshow(k, frames[k].frame)
                if cv2.waitKey(1) & 0xFF == ord('q'):
                    break
            
            print(f"collected {collected_frames}/{frames_to_collect} frames")
        print(f"fps: {frames_to_collect / time_taken}")
        
        
    except Exception as e:
        raise e
    finally:
        multi_sender.stop_processes()
        receiver.stop()
        cv2.destroyAllWindows()