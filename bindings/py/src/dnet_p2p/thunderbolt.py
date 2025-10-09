"""
Thunderbolt-related classes for DnetP2P library.
"""

from typing import List, Tuple, Optional, Mapping
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

    ip_addr: str
    """Thunderbolt IP address of this device."""

    instances: List[Tuple[ThunderboltInstance, List[ThunderboltInstance]]]
    """
    Domain UUIDs of Thunderbolt ports and their connections.
    Each tuple contains:
    - First element: The port/host instance
    - Second element: List of connected device instances
    """


class ThunderboltProperties(BaseModel):
    """Model representing Thunderbolt properties of a device.
    
    Shall be mixed-in with other device properties."""
    thunderbolt: Optional[ThunderboltData] = None

class ThunderboltConnection(BaseModel):
    """Model representing a Thunderbolt connection to another device."""

    ip_addr: str
    """IP address of the connected device."""

    instance: ThunderboltInstance
    """The Thunderbolt instance of the connected device."""

def discover_thunderbolt_connections(
    devices: Mapping[str, ThunderboltProperties],
) -> dict[str, dict[str, ThunderboltConnection]]:
    """
    Discover Thunderbolt connections based on the given devices.

    From the given devices, it scans for matching uuids in the Thunderbolt
    instances and builds a mapping of connections.

    Returns:
        A map in the form of `service_A_name -> (service_B_name -> ThunderboltConnection)`
        indicating that service A can reach service B via Thunderbolt at the given IP address.
    """
    ans = {}

    for this_name, this in devices.items():
        if not this.thunderbolt:
            continue  # skip missing thunderbolt info
        conns = {}

        # iterate over all other devices
        for other_name, other in devices.items():
            if not other.thunderbolt:
                continue  # skip missing thunderbolt info
            if this_name == other_name:
                continue  # skip self

            # check if the connected instances of the other devices
            # have a matching uuid in this device's instances
            for other_instance, other_connecteds in other.thunderbolt.instances:
                for other_connection in other_connecteds:
                    for this_instance, _ in this.thunderbolt.instances:
                        if other_connection.uuid == this_instance.uuid:
                            # found a match, use the first IP address of the other device
                            conns[other_name] = ThunderboltConnection(
                                ip_addr=other.thunderbolt.ip_addr,
                                instance=other_instance,
                            )
                            break

        # record connections if there were any
        if len(conns) > 0:
            ans[this_name] = conns

    return ans
