import cv2
import time
import argparse
import numpy as np

from progress.bar import Bar
from logging import getLogger, basicConfig
from traceback import format_exc

# ---------------------------------------------------------------------

AP = argparse.ArgumentParser()
AP.add_argument("--config", type=str, default="./configs/devices.json", help="path to device config file")
AP.add_argument("--proxy_sub_port", type=int, default=10000, help="port for proxy subscriber")
AP.add_argument("--proxy_pub_port", type=int, default=10001, help="port for proxy publisher")
AP.add_argument("--host", type=str, default="127.0.0.1", help="host name or ip of the server")
AP.add_argument("-ll", "--logging_level", type=str, default="info", help="logging level", choices=["debug", "warning", "error"])
ARGS = AP.parse_args()

# ---------------------------------------------------------------------

basicConfig(level=ARGS.logging_level.upper())
logger = getLogger(__name__)

# ---------------------------------------------------------------------

# Setp 1: read camera configurations for all cameras
from device_capture_system.core import MultiInputStreamSender, InputStreamReceiver
from device_capture_system.deviceIO import load_all_devices_from_config

if __name__ == "__main__":
    
    microphones = load_all_devices_from_config("audio", config_file=ARGS.config)
    
    for mic in microphones:
        print(mic)
    
    multi_sender = MultiInputStreamSender(
        devices = microphones,
        proxy_sub_port=ARGS.proxy_sub_port,
        proxy_pub_port=ARGS.proxy_pub_port,
        host = ARGS.host
    )
    
    receiver = InputStreamReceiver(
        devices = microphones,
        proxy_pub_port = ARGS.proxy_pub_port,
        host = ARGS.host
    )
    
    mag_bar = Bar("Mag", max=180)
    
    try:
        
        receiver.start()
        multi_sender.start_processes()
        
        time_taken = 0
        collected_frames = 0
        frames_to_collect = 180
        while collected_frames < frames_to_collect:
            dt = time.time()
            
            frames = receiver.read()
            
            if frames is None:
                continue
            
            time_taken += time.time() - dt
            collected_frames += 1
            
            for k in frames:
                frame = frames[k].frame.squeeze()
                
                mean_abs = np.abs(frame).mean()
                mag_bar.goto(int(mean_abs))
            
            print(f"collected {collected_frames} frames")
        print(f"fps: {frames_to_collect / time_taken}")
        
        
    except Exception as e:
        raise e
    finally:
        multi_sender.stop_processes()
        receiver.stop()
        mag_bar.finish()