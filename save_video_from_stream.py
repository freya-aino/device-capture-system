import argparse
import logging
import argparse

# ---------------------------------------------------------------------

AP = argparse.ArgumentParser()
AP.add_argument("-cc", "--cameras_config", type=str, default="./cameras_configs.json", help="path to input configuration file")
AP.add_argument("-hn", "--host_name", type=str, default="127.0.0.1", help="host name or ip of the server")

AP.add_argument("-op", "--output_path", type=str, required=True, help="output path")
AP.add_argument("-of", "--output_format", type=str, default="mp4", help="output format")
AP.add_argument("--fps", type=int, default=30, help="frames per second")
AP.add_argument("-vl", "--video_length_seconds", type=int, default=10, help="video length in seconds")

AP.add_argument("-ll", "--logging_level", type=str, default="info", help="logging level", choices=["debug", "warning", "error"])
ARGS = AP.parse_args()

# ---------------------------------------------------------------------

logging.basicConfig(level=ARGS.logging_level.upper())
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------

from camera_capture_system.core import load_all_cameras_from_config, MultiCapturePublisher
from camera_capture_system.fileIO import CaptureVideoSaver
from camera_capture_system.datamodel import VideoParameters

if __name__ == "__main__":
    cameras = load_all_cameras_from_config(ARGS.cameras_config)
    
    logger.warning(f"currently only mp4 format is supported for video saving.")
    assert ARGS.output_format == "mp4", "exiting because non mp4 fofmat ..."
    
    vp = VideoParameters(
        save_path = ARGS.output_path,
        fps = ARGS.fps,
        seconds = ARGS.video_length_seconds,
        codec = "mp4v", # todo: add more encoding options
        output_format = ARGS.output_format
    )
    
    mcp = MultiCapturePublisher(
        cameras=cameras, 
        host=ARGS.host_name,
        frame_transforms={
            "cam0": "ROTATE_90_COUNTERCLOCKWISE",
            "cam1": "ROTATE_90_COUNTERCLOCKWISE",
            "cam2": "ROTATE_90_CLOCKWISE"
        })
    
    cis = CaptureVideoSaver(
        cameras=cameras,
        video_params=vp,
        host=ARGS.host_name
    )
    
    from time import sleep
    
    try:
        
        mcp.start()
        cis.start()
        
        while True:
            cis.save_video()
            sleep(1)
        
    except KeyboardInterrupt:
        logger.info("KeyboardInterrupt ...")
    except:
        raise
    finally:
        mcp.stop()
        cis.stop()
