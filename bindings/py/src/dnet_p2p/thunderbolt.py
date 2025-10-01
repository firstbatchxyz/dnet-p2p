"""
Thunderbolt-related classes for DnetP2P library.
"""

from typing import List, Tuple
from pydantic import BaseModel


class ThunderboltInstance(BaseModel):
    """Model representing a Thunderbolt instance/device."""

    uuid: str
    """Domain UUID of the device, from `domain_uuid_key`."""

    name: str
    """Name of the connection, e.g. "thunderboltusb4_bus_2" or "Macbook Air", from `_name`."""

    device: str
    """Human-readable name of the device, e.g. "Mac15,12", from `device_name_key`."""


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
