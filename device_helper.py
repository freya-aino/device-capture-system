import argparse
import sys
from typing import List

from device_capture_system.deviceIO import (
    get_all_devices_ffmpeg, 
    save_periphery_devices_to_config,
    get_video_device_configurations,
    get_audio_device_configurations
)
from device_capture_system.datamodel import PeripheryDevice

AP = argparse.ArgumentParser()
AP.add_argument("-p", "--print_devices", action="store_true", help="print devices to console")
AP.add_argument("-v", "--verbose", action="store_true", help="print debug messages")
AP.add_argument("-s", "--save_devices", action="store_true", help="save devices to config")
AP.add_argument("-c", "--configure", action="store_true", help="configure devices interactively")
AP.add_argument("-o", "--output_path", type=str, default=None, help="output path")
ARGS = AP.parse_args()


if len(sys.argv) == 1:
    AP.print_help(sys.stderr)
    sys.exit(1)

# ---------------------------------------------------------------------

def device__config_selection(device_configurations: List[PeripheryDevice]) -> PeripheryDevice:
    # pretty print devices and ask to select one of them
    
    print(f"\nplease select device configurations for:")
    print(f"\tname: {device.name}")
    print(f"\tdevice_id: {device.device_id}")
    print(f"\tdevice_type: {device.device_type}")
    for i, cfg in enumerate(device_configurations):
        print("".join([f"{i}. {' || '.join([f'{k}={v}' for k, v in cfg.model_dump().items() if k != 'device_id' and k != 'name' and k != 'device_type' ])}"]))

    cfg_idx = int(input("Select configuration: "))
    assert 0 <= cfg_idx < len(device_configurations), "invalid configuration index"
    return device_configurations[cfg_idx]

# ---------------------------------------------------------------------

if __name__ == "__main__":
    devices = get_all_devices_ffmpeg()
    
    if ARGS.print_devices:
        print("Video Devices:\n")
        for video_device in [dev for dev in devices if dev.device_type == "video"]:
            if ARGS.verbose:
                print("\t", video_device.name, " : ", video_device.device_id)
            else:
                print("\t", video_device.name)
        
        print("\nAudio Devices:\n")
        for audio_device in [dev for dev in devices if dev.device_type == "audio"]:
            if ARGS.verbose:
                print("\t", audio_device.name, " : ", audio_device.device_id)
            else:
                print("\t", audio_device.name)
    
    if ARGS.configure:
        for device in devices:
            if device.device_type == "video":
                device_configurations = get_video_device_configurations(device)

                if len(device_configurations) == 0:
                    print(f"No configurations found for {device.name}")
                    continue

                selected_device = device__config_selection(device_configurations)

            elif device.device_type == "audio":

                device_configurations = get_audio_device_configurations(device)

                if len(device_configurations) == 0:
                    print(f"No configurations found for {device.name}")
                    continue

                selected_device = device__config_selection(device_configurations)

    if ARGS.save_devices:
        assert ARGS.output_path is not None, "output path must be provided"
        assert ARGS.output_path.endswith(".json"), "output path must be a json file"
        save_periphery_devices_to_config(devices, ARGS.output_path)