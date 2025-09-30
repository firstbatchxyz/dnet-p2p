"""
Thunderbolt-specific discovery & connection logic.

This is MacOS focused, and uses the following command:

```sh
system_profiler SPNetworkDataType SPThunderboltDataType -json
```
"""

import json
from typing import Dict, List, Optional, Set
from dataclasses import dataclass, field


@dataclass
class ThunderboltDevice:
    """Represents a Thunderbolt device."""

    name: str
    device_name: str
    domain_uuid: str
    vendor_name: str
    device_id: Optional[str] = None
    vendor_id: Optional[str] = None
    route_string: Optional[str] = None
    switch_uid: Optional[str] = None
    services: List[Dict] = field(default_factory=list)


@dataclass
class ThunderboltBus:
    """Represents a Thunderbolt bus (host) with its connected devices."""

    name: str
    device_name: str
    domain_uuid: str
    vendor_name: str
    switch_uid: str
    route_string: str
    receptacle_id: Optional[str] = None
    receptacle_status: Optional[str] = None
    link_status: Optional[str] = None
    current_speed: Optional[str] = None
    connected_devices: List[ThunderboltDevice] = field(default_factory=list)


class ThunderboltJSONParser:
    def __init__(self, json_data: Dict):
        """
        Initialize parser with JSON data.

        Args:
            json_data: Parsed JSON dictionary from system_profiler output
        """
        self.json_data = json_data
        self.buses: List[ThunderboltBus] = []
        self.parse()

    @classmethod
    def from_file(cls, filepath: str) -> "ThunderboltJSONParser":
        """Create parser from a JSON file."""
        with open(filepath, "r") as f:
            data = json.load(f)
        return cls(data)

    @classmethod
    def from_string(cls, json_string: str) -> "ThunderboltJSONParser":
        """Create parser from a JSON string."""
        data = json.loads(json_string)
        return cls(data)

    def parse(self):
        """Parse the JSON data and extract Thunderbolt information."""
        thunderbolt_data = self.json_data.get("SPThunderboltDataType", [])

        for bus_entry in thunderbolt_data:
            bus = self._parse_bus(bus_entry)
            if bus:
                self.buses.append(bus)

    def _parse_bus(self, bus_entry: Dict) -> Optional[ThunderboltBus]:
        """Parse a Thunderbolt bus entry."""
        # Extract receptacle information
        receptacle_key = next(
            (key for key in bus_entry if key.startswith("receptacle_")), None
        )
        receptacle_data = bus_entry.get(receptacle_key, {}) if receptacle_key else {}

        bus = ThunderboltBus(
            name=bus_entry.get("_name", ""),
            device_name=bus_entry.get("device_name_key", ""),
            domain_uuid=bus_entry.get("domain_uuid_key", ""),
            vendor_name=bus_entry.get("vendor_name_key", ""),
            switch_uid=bus_entry.get("switch_uid_key", ""),
            route_string=bus_entry.get("route_string_key", ""),
            receptacle_id=receptacle_data.get("receptacle_id_key"),
            receptacle_status=receptacle_data.get("receptacle_status_key"),
            link_status=receptacle_data.get("link_status_key"),
            current_speed=receptacle_data.get("current_speed_key"),
        )

        # Parse connected devices
        items = bus_entry.get("_items", [])
        for item in items:
            device = self._parse_device(item)
            if device:
                bus.connected_devices.append(device)

        return bus

    def _parse_device(self, device_entry: Dict) -> Optional[ThunderboltDevice]:
        """Parse a connected device entry."""
        services = device_entry.get("services_title", [])

        device = ThunderboltDevice(
            name=device_entry.get("_name", ""),
            device_name=device_entry.get("device_name_key", ""),
            domain_uuid=device_entry.get("domain_uuid_key", ""),
            vendor_name=device_entry.get("vendor_name_key", ""),
            device_id=device_entry.get("device_id_key"),
            vendor_id=device_entry.get("vendor_id_key"),
            route_string=device_entry.get("route_string_key"),
            switch_uid=device_entry.get("switch_uid_key"),
            services=services,
        )

        return device

    def get_domain_uuids(self) -> Set[str]:
        """Get all domain UUIDs from this profile (buses and devices)."""
        uuids = set()

        for bus in self.buses:
            if bus.domain_uuid:
                uuids.add(bus.domain_uuid)

            for device in bus.connected_devices:
                if device.domain_uuid:
                    uuids.add(device.domain_uuid)

        return uuids

    def get_all_devices(self) -> List[ThunderboltDevice]:
        """Get all connected devices from all buses."""
        devices = []
        for bus in self.buses:
            devices.extend(bus.connected_devices)
        return devices

    def __str__(self) -> str:
        """String representation of the parsed data."""
        result = []
        for bus in self.buses:
            result.append(f"Bus: {bus.name}")
            result.append(f"  Device: {bus.device_name}")
            result.append(f"  Domain UUID: {bus.domain_uuid}")
            result.append(f"  Status: {bus.receptacle_status}")
            result.append(f"  Speed: {bus.current_speed}")

            if bus.connected_devices:
                result.append("  Connected devices:")
                for device in bus.connected_devices:
                    result.append(f"    - {device.name} ({device.device_name})")
                    result.append(f"      Domain UUID: {device.domain_uuid}")
                    result.append(f"      Vendor: {device.vendor_name}")
            else:
                result.append("  No connected devices")
            result.append("")

        return "\n".join(result)


@dataclass
class ThunderboltConnection:
    """Represents a discovered connection between two devices."""

    device1_name: str
    device1_uuid: str
    device1_bus_name: str
    device2_name: str
    device2_uuid: str
    device2_bus_name: str
    connection_type: str  # "bidirectional" or "one-way"


def discover_connections(
    parser1: ThunderboltJSONParser, parser2: ThunderboltJSONParser
) -> List[ThunderboltConnection]:
    """
    Discover Thunderbolt connections between two device profiles.

    Connections are identified by matching domain_uuid values where:
    - A device in profile1 has a domain_uuid that matches a bus in profile2
    - A device in profile2 has a domain_uuid that matches a bus in profile1

    Args:
        parser1: First device's Thunderbolt parser
        parser2: Second device's Thunderbolt parser

    Returns:
        List of discovered connections
    """
    connections = []
    found_pairs = set()  # Track UUID pairs to avoid duplicates

    # Get all UUIDs from both profiles
    uuids1 = parser1.get_domain_uuids()
    uuids2 = parser2.get_domain_uuids()

    # Check devices in profile1 against buses in profile2
    for bus1 in parser1.buses:
        for device1 in bus1.connected_devices:
            if not device1.domain_uuid:
                continue

            # Check if this device's UUID matches a bus in profile2
            for bus2 in parser2.buses:
                if device1.domain_uuid == bus2.domain_uuid:
                    # Found a match! Check if bidirectional
                    is_bidirectional = False

                    # Look for reverse connection
                    for device2 in bus2.connected_devices:
                        if device2.domain_uuid == bus1.domain_uuid:
                            is_bidirectional = True
                            break

                    pair_key = tuple(sorted([bus1.domain_uuid, bus2.domain_uuid]))
                    if pair_key not in found_pairs:
                        found_pairs.add(pair_key)
                        connections.append(
                            ThunderboltConnection(
                                device1_name=bus1.device_name,
                                device1_uuid=bus1.domain_uuid,
                                device1_bus_name=bus1.name,
                                device2_name=bus2.device_name,
                                device2_uuid=bus2.domain_uuid,
                                device2_bus_name=bus2.name,
                                connection_type="bidirectional"
                                if is_bidirectional
                                else "one-way",
                            )
                        )

    return connections


def main():
    """Main function to demonstrate the parser."""
    import sys

    if len(sys.argv) < 3:
        print(
            "Usage: python thunderbolt_json_parser.py <profile1.json> <profile2.json>"
        )
        print("\nExample:")
        print("  python thunderbolt_json_parser.py profile1.json profile2.json")
        sys.exit(1)

    profile1_path = sys.argv[1]
    profile2_path = sys.argv[2]

    try:
        # Parse both profiles
        print(f"Parsing {profile1_path}...")
        parser1 = ThunderboltJSONParser.from_file(profile1_path)
        print(f"\n=== Profile 1 ({profile1_path}) ===")
        print(parser1)

        print(f"Parsing {profile2_path}...")
        parser2 = ThunderboltJSONParser.from_file(profile2_path)
        print(f"\n=== Profile 2 ({profile2_path}) ===")
        print(parser2)

        # Discover connections
        print("\n=== Connection Discovery ===")
        connections = discover_connections(parser1, parser2)

        if connections:
            print(f"\nFound {len(connections)} Thunderbolt connection(s):\n")
            for i, conn in enumerate(connections, 1):
                print(f"Connection {i} ({conn.connection_type}):")
                print(f"  Device 1: {conn.device1_name}")
                print(f"    Bus: {conn.device1_bus_name}")
                print(f"    UUID: {conn.device1_uuid}")
                print()
                print(f"  Device 2: {conn.device2_name}")
                print(f"    Bus: {conn.device2_bus_name}")
                print(f"    UUID: {conn.device2_uuid}")
                print()
        else:
            print("No Thunderbolt connections found between the devices.")

    except FileNotFoundError as e:
        print(f"Error: Could not find file - {e}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON format - {e}")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
