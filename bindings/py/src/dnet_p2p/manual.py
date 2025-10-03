"""
Utility to allow a user to specify a list of devices manually via a JSON file.

This is expected to be used in conjunction with the mDNS discovery, and the two lists
will be merged together, prioritizing the manually specified devices in case of
duplicates.

This makes sense for API in particular, instead of shards. The API can discover other devices
via mDNS etc. and can combine them with the manually specified devices. Shards, on the other hand,
are expected to be run in more controlled environments, and typically do not need to know about
other devices beyond the ones they are directly communicating with (e.g., an API server).

The API can run the solver etc. to later let the shards know about the manually given devices, if needed.
"""

import json
from pathlib import Path

from .core import DnetDeviceProperties


def load_manual_devices(path: str | Path) -> dict[str, DnetDeviceProperties]:
    """
    Load DnetDeviceProperties instances from a JSON file.

    This is mostly useful for the API side, which can take in multiple devices that
    do not necessarily advertise themselves via mDNS. For example, an AWS instance
    can be specified via its private IP address from this file, assuming that it is
    running the shard service.

    Args:
        path: Path to the JSON file containing an array of device properties

    Returns:
        list[DnetDeviceProperties]: List of validated device properties

    Raises:
        FileNotFoundError: If the file does not exist
        json.JSONDecodeError: If the file is not valid JSON
        ValueError: If the JSON is not an array or validation fails
    """
    path = Path(path)

    if not path.exists():
        raise FileNotFoundError(f"Device list file not found: {path}")

    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)

    if not isinstance(data, list):
        raise ValueError("Device list must be a JSON array of device properties")

    devices = {}
    for entry in data:
        try:
            device = DnetDeviceProperties(**entry)
            devices[f"manual_{device.instance}"] = device
        except Exception as e:
            raise ValueError(f"Invalid device entry: {entry}") from e

    return devices


def merge_device_mappings(
    via_mdns: dict[str, DnetDeviceProperties],
    via_manual: dict[str, DnetDeviceProperties],
) -> dict[str, DnetDeviceProperties]:
    """
    Merge multiple mappings of DnetDeviceProperties, prioritizing the entries in `via_manual` over
    those in `via_mdns`. To detect duplicates, the `instance` and `host` fields are used as the unique identifier.

    Args:
        via_mdns: Existing mapping of device names to properties discovered via mDNS
        via_manual: New mapping of device names to properties loaded from a devices file

    Returns:
        dict[str, DnetDeviceProperties]: A dictionary mapping device names to their properties
    """

    # get device keys based on (instance, host) tuples from the manual list
    devices = via_manual.copy()

    # create a set of existing (instance, host) tuples to detect duplicates
    device_keys = {(device.instance, device.host) for device in via_mdns.values()}

    # add non-duplicate devices from the mDNS to the existing mapping
    for name, device in via_mdns.items():
        if (device.instance, device.host) not in device_keys:
            devices[name] = device

    return devices
