"""
Thunderbolt-related classes for DnetP2P library.
"""

from typing import List, Tuple, Optional, Dict, Any
from pydantic import BaseModel


class ThunderboltInstance(BaseModel):
    """Model representing a Thunderbolt instance/device."""

    uuid: str
    """Domain UUID of the device, from `domain_uuid_key`."""

    name: str
    """Name of the connection, e.g. "thunderboltusb4_bus_2" or "Macbook Air", from `_name`."""

    device: str
    """Human-readable name of the device, e.g. "Mac15,12", from `device_name_key`."""

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Optional["ThunderboltInstance"]:
        """Create a ThunderboltInstance from a dictionary representation."""
        try:
            return cls(
                uuid=data.get("domain_uuid_key", ""),
                name=data.get("_name", ""),
                device=data.get("device_name_key", ""),
            )
        except Exception:
            return None


class ThunderboltData(BaseModel):
    """Model representing Thunderbolt connection information."""

    ip_addrs: List[str]
    """Thunderbolt IP addresses from the Thunderbolt Bridge interface."""

    instances: List[Tuple[ThunderboltInstance, List[ThunderboltInstance]]]
    """
    Domain UUIDs of Thunderbolt ports and their connections.
    Each tuple contains:
    - First element: The port/host instance
    - Second element: List of connected device instances
    """

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Optional["ThunderboltData"]:
        """Create ThunderboltData from a dictionary representation (typically from JSON)."""
        try:
            # Extract Thunderbolt IP addresses from network information
            ip_addrs = []
            if "SPNetworkDataType" in data:
                for item in data["SPNetworkDataType"]:
                    if item.get("_name") == "Thunderbolt Bridge":
                        ip_addrs = item.get("ip_address", [])
                        break

            # Extract Thunderbolt instances and connections
            instances = []
            if "SPThunderboltDataType" in data:
                for item in data["SPThunderboltDataType"]:
                    # Create the host/port instance
                    host_instance = ThunderboltInstance.from_dict(item)
                    if host_instance:
                        connected_devices = []

                        # Check if this port has connected devices in `_items`
                        if "_items" in item and isinstance(item["_items"], list):
                            for connected_item in item["_items"]:
                                device_instance = ThunderboltInstance.from_dict(
                                    connected_item
                                )
                                if device_instance:
                                    connected_devices.append(device_instance)

                        instances.append((host_instance, connected_devices))

            return cls(ip_addrs=ip_addrs, instances=instances)

        except Exception:
            return None
