from .thunderbolt import (
    ThunderboltInstance,
    ThunderboltData,
    ThunderboltProperties,
    discover_all_thunderbolt_connections,
)

DEVICE_A_IP = "164.259.0.1"
DEVICE_B_IP = "164.259.0.2"
DEVICE_C_IP = "164.259.0.3"


def test_discover_thunderbolt_connections_basic():
    """Test basic Thunderbolt connection discovery."""
    # Create mock devices with Thunderbolt connections
    device_a_uuid = "uuid-a-port-1"
    device_b_uuid = "uuid-b-port-1"

    # Device A has a port with uuid-a-port-1, and it's connected to device B's uuid-b-port-1
    device_a = ThunderboltProperties(
        thunderbolt=ThunderboltData(
            ip_addr=DEVICE_A_IP,
            instances=[
                (
                    ThunderboltInstance(
                        uuid=device_a_uuid, name="port-a-1", device="DeviceA"
                    ),
                    [
                        ThunderboltInstance(
                            uuid=device_b_uuid, name="port-b-1", device="DeviceB"
                        )
                    ],
                )
            ],
        )
    )

    # Device B has a port with uuid-b-port-1, and it's connected to device A's uuid-a-port-1
    device_b = ThunderboltProperties(
        thunderbolt=ThunderboltData(
            ip_addr=DEVICE_B_IP,
            instances=[
                (
                    ThunderboltInstance(
                        uuid=device_b_uuid, name="port-b-1", device="DeviceB"
                    ),
                    [
                        ThunderboltInstance(
                            uuid=device_a_uuid, name="port-a-1", device="DeviceA"
                        )
                    ],
                )
            ],
        )
    )

    devices = {"device-a": device_a, "device-b": device_b}

    # Discover connections
    connections = discover_all_thunderbolt_connections(devices)

    # Both devices should see each other
    assert "device-a" in connections
    assert "device-b" in connections

    # Device A should see Device B
    assert "device-b" in connections["device-a"]
    device_b_a_conn = connections["device-a"]["device-b"]
    assert device_b_a_conn.ip_addr == DEVICE_B_IP
    assert device_b_a_conn.instance.uuid == device_b_uuid

    # Device B should see Device A
    assert "device-a" in connections["device-b"]
    device_b_a_conn = connections["device-b"]["device-a"]
    assert device_b_a_conn.ip_addr == DEVICE_A_IP
    assert device_b_a_conn.instance.uuid == device_a_uuid


def test_discover_thunderbolt_connections_no_thunderbolt():
    """Test discovery with devices that don't have Thunderbolt."""
    device_a = ThunderboltProperties(thunderbolt=None)
    device_b = ThunderboltProperties(thunderbolt=None)

    devices = {"device-a": device_a, "device-b": device_b}
    connections = discover_all_thunderbolt_connections(devices)

    # No connections should be found
    assert len(connections) == 0


def test_discover_thunderbolt_connections_no_match():
    """Test discovery when devices don't match."""
    device_a = ThunderboltProperties(
        thunderbolt=ThunderboltData(
            ip_addr=DEVICE_A_IP,
            instances=[
                (
                    ThunderboltInstance(uuid="uuid-a", name="port-a", device="DeviceA"),
                    [],  # No connections
                )
            ],
        )
    )

    device_b = ThunderboltProperties(
        thunderbolt=ThunderboltData(
            ip_addr=DEVICE_B_IP,
            instances=[
                (
                    ThunderboltInstance(uuid="uuid-b", name="port-b", device="DeviceB"),
                    [],  # No connections
                )
            ],
        )
    )

    devices = {"device-a": device_a, "device-b": device_b}
    connections = discover_all_thunderbolt_connections(devices)

    # No connections should be found
    assert len(connections) == 0


def test_discover_thunderbolt_connections_multiple_devices():
    """Test discovery with multiple devices in a chain."""
    uuid_a, uuid_b, uuid_c = "uuid-a", "uuid-b", "uuid-c"

    # device A connected to B
    device_a = ThunderboltProperties(
        thunderbolt=ThunderboltData(
            ip_addr=DEVICE_A_IP,
            instances=[
                (
                    ThunderboltInstance(uuid=uuid_a, name="port-a", device="DeviceA"),
                    [ThunderboltInstance(uuid=uuid_b, name="port-b", device="DeviceB")],
                )
            ],
        )
    )

    # device B connected to both A and C
    device_b = ThunderboltProperties(
        thunderbolt=ThunderboltData(
            ip_addr=DEVICE_B_IP,
            instances=[
                (
                    ThunderboltInstance(uuid=uuid_b, name="port-b", device="DeviceB"),
                    [
                        # fmt: off
                        ThunderboltInstance(
                            uuid=uuid_a, name="port-a", device="DeviceA"
                        ),
                        ThunderboltInstance(
                            uuid=uuid_c, name="port-c", device="DeviceC"
                        ),
                        # fmt: on
                    ],
                )
            ],
        )
    )

    # Device C connected to B
    device_c = ThunderboltProperties(
        thunderbolt=ThunderboltData(
            ip_addr=DEVICE_C_IP,
            instances=[
                (
                    ThunderboltInstance(uuid=uuid_c, name="port-c", device="DeviceC"),
                    [ThunderboltInstance(uuid=uuid_b, name="port-b", device="DeviceB")],
                )
            ],
        )
    )

    devices = {"device-a": device_a, "device-b": device_b, "device-c": device_c}
    connections = discover_all_thunderbolt_connections(devices)

    # Verify connections
    assert len(connections) == 3

    # Device A sees B
    assert "device-b" in connections["device-a"]
    assert connections["device-a"]["device-b"].ip_addr == DEVICE_B_IP

    # Device B sees both A and C
    assert "device-a" in connections["device-b"]
    assert "device-c" in connections["device-b"]
    assert connections["device-b"]["device-a"].ip_addr == DEVICE_A_IP
    assert connections["device-b"]["device-c"].ip_addr == DEVICE_C_IP

    # Device C sees B
    assert "device-b" in connections["device-c"]
    assert connections["device-c"]["device-b"].ip_addr == DEVICE_B_IP
