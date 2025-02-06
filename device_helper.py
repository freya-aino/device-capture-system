# import argparse
# import sys
# from typing import List
# import os

# from device_capture_system.deviceIO import (
#     get_all_devices_ffmpeg, 
#     get_video_device_configurations,
#     get_audio_device_configurations,
#     parse_device_configurations,
#     save_periphery_devices_to_config,
# )
# from device_capture_system.datamodel import PeripheryDevice

# AP = argparse.ArgumentParser()
# AP.add_argument("-p", "--print_devices", action="store_true", help="print devices to console")
# AP.add_argument("-v", "--verbose", action="store_true", help="print debug messages")
# AP.add_argument("-c", "--configure", action="store_true", help="configure devices interactively and save to file")
# ARGS = AP.parse_args()


# if len(sys.argv) == 1:
#     AP.print_help(sys.stderr)
#     sys.exit(1)

# # ---------------------------------------------------------------------

# def device_config_selection(device_configurations: List[PeripheryDevice]) -> PeripheryDevice:
#     # pretty print devices and ask to select one of them
    
#     print(f"\nplease select device configurations for:")
#     print(f"\tname: {device.name}")
#     print(f"\tdevice_id: {device.device_id}")
#     print(f"\tdevice_type: {device.device_type}")
#     for i, cfg in enumerate(device_configurations):
#         print("".join([f"{i}. {' || '.join([f'{k}={v}' for k, v in cfg.model_dump().items() if k != 'device_id' and k != 'name' and k != 'device_type' ])}"]))

#     try:
#         cfg_idx = int(input("Select configuration: "))
#         assert 0 <= cfg_idx < len(device_configurations), "invalid configuration index"
#         return device_configurations[cfg_idx]
#     except ValueError as e:
#         print(f"invalid input: {e}")
#         raise e


# # ---------------------------------------------------------------------

# if __name__ == "__main__":
#     devices = get_all_devices_ffmpeg()
    
#     if ARGS.print_devices:
#         print("Video Devices:\n")
#         for video_device in [dev for dev in devices if dev.device_type == "video"]:
#             if ARGS.verbose:
#                 print("\t", video_device.name, " : ", video_device.device_id)
#             else:
#                 print("\t", video_device.name)
        
#         print("\nAudio Devices:\n")
#         for audio_device in [dev for dev in devices if dev.device_type == "audio"]:
#             if ARGS.verbose:
#                 print("\t", audio_device.name, " : ", audio_device.device_id)
#             else:
#                 print("\t", audio_device.name)
    
#     if ARGS.configure:
        
#         selected_device_configurations = []

#         for device in devices:
#             if device.device_type == "video":
#                 device_configurations = get_video_device_configurations(device)
#             elif device.device_type == "audio":
#                 device_configurations = get_audio_device_configurations(device)
#             else:
#                 print(f"Unknown device type: {device.name} - {device.device_type}")

#             if len(device_configurations) == 0:
#                 print(f"No configurations found for {device.name}")
#                 continue

#             selected_device_configurations.append(device_config_selection(device_configurations))

#         parsed_devices = []
#         for config in selected_device_configurations:
#             parsed_devices.append(config)

#         print("\nSelected Configurations:\n")
#         for cfg in parsed_devices:
#             print(cfg)
#             print("\n")

#         # option to choose output path or leave default
#         output_path = input(f"selected output path [./configs/devices.json]:")
#         if output_path == "":
#             output_path = "./configs/devices.json"
#         assert output_path.endswith(".json"), "output path must be a json file"
#         assert os.path.exists(os.path.dirname(output_path)), "output path directory does not exist"

#         save_periphery_devices_to_config(parsed_devices, output_path)