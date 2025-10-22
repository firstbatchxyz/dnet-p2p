"""
Thunderbolt-related classes for DnetP2P library.
"""

from typing import List, Tuple, Optional, Mapping
from pydantic import BaseModel, Field


class ThunderboltInstance(BaseModel):
    """Model representing a Thunderbolt instance/device."""

    uuid: str = Field(
        ..., description="Domain UUID of the device, from `domain_uuid_key`."
    )

    name: str = Field(
        ...,
        description="Name of the connection, e.g. 'thunderboltusb4_bus_2' or 'Macbook Air', from `_name`.",
    )

    device: str = Field(
        ...,
        description="Human-readable name of the device, e.g. 'Mac15,12', from `device_name_key`.",
    )


class ThunderboltData(BaseModel):
    """Model representing Thunderbolt connection information."""

    ip_addr: str = Field(..., description="Thunderbolt IP address of this device.")

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

    thunderbolt: Optional[ThunderboltData] = Field(default=None)


class ThunderboltConnection(BaseModel):
    """Model representing a Thunderbolt connection to another device."""

    ip_addr: str = Field(..., description="IP address of the connected device.")

    instance: ThunderboltInstance = Field(
        ..., description="The Thunderbolt instance of the connected device."
    )


def discover_all_thunderbolt_connections(
    devices: Mapping[str, ThunderboltProperties],
) -> dict[str, dict[str, ThunderboltConnection]]:
    """
    Discover all Thunderbolt connections based on the given devices.

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

            conn = discover_thunderbolt_connection(this, other)
            if conn:
                conns[other_name] = conn

        # record connections if there were any
        if len(conns) > 0:
            ans[this_name] = conns

    return ans


def discover_thunderbolt_connection(
    this: ThunderboltProperties, other: ThunderboltProperties
) -> Optional[ThunderboltConnection]:
    """
    Discover a Thunderbolt connection between two devices.

    Scans for matching uuids in the Thunderbolt instances of the two devices
    and returns a `ThunderboltConnection` if a connection is found.

    Args:
        this: The Thunderbolt properties of the first device.
        other: The Thunderbolt properties of the second device.

    Returns:
        A `ThunderboltConnection` if a connection is found, otherwise `None`.
    """
    if not this.thunderbolt or not other.thunderbolt:
        return None  # missing thunderbolt info

    for other_instance, other_connecteds in other.thunderbolt.instances:
        for other_connection in other_connecteds:
            for this_instance, _ in this.thunderbolt.instances:
                if other_connection.uuid == this_instance.uuid:
                    # found a match!

                    # edge case check: other device could be another shard within the same "machine",
                    # we can compare both IPs for sanity, to disallow self-thunderbolt connections
                    if this.thunderbolt.ip_addr == other.thunderbolt.ip_addr:
                        # same IP, likely the same machine, skip
                        continue

                    return ThunderboltConnection(
                        ip_addr=other.thunderbolt.ip_addr,
                        instance=other_instance,
                    )

    return None  # no connection found
