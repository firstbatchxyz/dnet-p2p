"""
Static peer discovery for CI/cloud environments.

This module provides a StaticDiscovery class that implements the same interface
as AsyncDnetP2P but reads peers from a hostfile instead of using UDP broadcast.

Useful for:
- CI/CD environments where UDP broadcast doesn't work
- Cloud deployments (AWS, GCP, Azure) across subnets
- Any environment where mDNS/UDP discovery fails
"""

from pathlib import Path
from typing import Literal, Optional

from .core import DnetDeviceProperties


def load_hostfile(path: Path | str) -> dict[str, DnetDeviceProperties]:
    """Load peers from a hostfile (SSH-style or JSON format).

    SSH-style format (one peer per line):
        # Comments start with #
        # Format: instance_name ip http_port grpc_port
        shard-1 10.0.1.100 8081 58081
        shard-2 10.0.1.101 8082 58082

    JSON format:
        [
          {"instance": "shard-1", "local_ip": "10.0.1.100", "server_port": 8081, "shard_port": 58081},
          {"instance": "shard-2", "local_ip": "10.0.1.101", "server_port": 8082, "shard_port": 58082}
        ]

    Args:
        path: Path to the hostfile

    Returns:
        dict[str, DnetDeviceProperties]: Mapping of instance names to properties

    Raises:
        FileNotFoundError: If the file does not exist
        ValueError: If the file format is invalid
    """
    import json

    path = Path(path)

    if not path.exists():
        raise FileNotFoundError(f"Hostfile not found: {path}")

    content = path.read_text(encoding="utf-8").strip()

    # Try JSON format first
    if content.startswith("["):
        try:
            data = json.loads(content)
            if not isinstance(data, list):
                raise ValueError("JSON hostfile must be an array")

            devices: dict[str, DnetDeviceProperties] = {}
            for entry in data:
                device = DnetDeviceProperties(**entry)
                devices[device.instance] = device
            return devices
        except json.JSONDecodeError as e:
            raise ValueError(f"Invalid JSON in hostfile: {e}") from e

    # Parse SSH-style format
    devices = {}
    for line_num, line in enumerate(content.splitlines(), start=1):
        line = line.strip()

        # Skip empty lines and comments
        if not line or line.startswith("#"):
            continue

        parts = line.split()
        if len(parts) != 4:
            raise ValueError(
                f"Invalid hostfile format at line {line_num}: expected "
                f"'instance_name ip http_port grpc_port', got: {line}"
            )

        instance_name, ip, http_port_str, grpc_port_str = parts

        try:
            http_port = int(http_port_str)
            grpc_port = int(grpc_port_str)
        except ValueError as e:
            raise ValueError(
                f"Invalid port number at line {line_num}: {e}"
            ) from e

        device = DnetDeviceProperties(
            instance=instance_name,
            local_ip=ip,
            server_port=http_port,
            shard_port=grpc_port,
            is_manager=False,
            is_busy=False,
        )
        devices[instance_name] = device

    return devices


class StaticDiscovery:
    """Static peer discovery for CI/cloud environments.

    Implements the same interface as AsyncDnetP2P but reads peers from a
    hostfile instead of using UDP broadcast. This is a drop-in replacement
    for AsyncDnetP2P when you know the peer addresses ahead of time.

    Usage:
        discovery = StaticDiscovery(
            hostfile=Path("hostfile"),
            own_instance="api-node",
            own_http_port=8080,
            own_grpc_port=58080,
        )
        # No need to call async_start() - it's a no-op
        devices = await discovery.async_get_properties()
    """

    def __init__(
        self,
        hostfile: Path | str,
        own_instance: str,
        own_http_port: int,
        own_grpc_port: int,
        own_ip: str = "127.0.0.1",
    ):
        """Initialize static discovery from a hostfile.

        Args:
            hostfile: Path to the hostfile containing peer definitions
            own_instance: This node's instance name
            own_http_port: This node's HTTP port
            own_grpc_port: This node's gRPC port
            own_ip: This node's IP address (default: 127.0.0.1)
        """
        self._own_instance = own_instance
        self._own_props = DnetDeviceProperties(
            instance=own_instance,
            local_ip=own_ip,
            server_port=own_http_port,
            shard_port=own_grpc_port,
            is_manager=True,  # API is always manager
            is_busy=False,
        )

        # Load peers from hostfile
        self._peers = load_hostfile(hostfile)
        self._running = False

    def instance_name(self) -> str:
        """Get the name of the current device's instance name."""
        return self._own_instance

    def is_running(self) -> bool:
        """Check if the discovery service is running."""
        return self._running

    def is_created(self) -> bool:
        """Check if an instance has been created."""
        return True  # Always "created" for static discovery

    async def async_start(
        self,
        loglevel: Optional[Literal["info", "debug", "trace", "warn", "error"]] = None,
    ) -> None:
        """Start the discovery service (no-op for static discovery)."""
        del loglevel  # Unused
        self._running = True

    async def async_stop(self) -> int:
        """Stop the discovery service (no-op for static discovery)."""
        self._running = False
        return 0

    async def async_get_properties(
        self, buffer_size: int = 4096
    ) -> dict[str, DnetDeviceProperties]:
        """Get the properties of discovered peers.

        Returns the static peer list loaded from the hostfile.

        Args:
            buffer_size: Ignored (kept for API compatibility)

        Returns:
            dict[str, DnetDeviceProperties]: Mapping of instance names to properties
        """
        del buffer_size  # Unused
        return self._peers

    async def async_get_own_properties(
        self, buffer_size: int = 4096
    ) -> DnetDeviceProperties:
        """Get this node's own properties.

        Args:
            buffer_size: Ignored (kept for API compatibility)

        Returns:
            DnetDeviceProperties: This node's properties
        """
        del buffer_size  # Unused
        return self._own_props

    async def async_set_is_busy(self, is_busy: bool) -> None:
        """Set the busy status of the service.

        Args:
            is_busy: Whether this node is busy
        """
        self._own_props.is_busy = is_busy

    async def async_free_instance(self) -> None:
        """Free resources (no-op for static discovery)."""
        self._running = False

    # Synchronous methods for compatibility

    def create_instance(
        self,
        instance: str,
        server_port: int,
        shard_port: int,
        is_manager: bool = False,
        is_passive: bool = False,
    ) -> None:
        """Create instance (no-op, already created in __init__)."""
        del instance, server_port, shard_port, is_manager, is_passive  # Unused
        pass

    def start(
        self,
        loglevel: Optional[Literal["info", "debug", "trace", "warn", "error"]] = None,
    ) -> None:
        """Start the discovery service (no-op)."""
        del loglevel  # Unused
        self._running = True

    def stop(self) -> int:
        """Stop the discovery service (no-op)."""
        self._running = False
        return 0

    def free_instance(self) -> None:
        """Free resources (no-op)."""
        self._running = False

    def get_properties(self, buffer_size: int = 4096) -> dict[str, DnetDeviceProperties]:
        """Synchronous version of async_get_properties."""
        del buffer_size  # Unused
        return self._peers

    def get_own_properties(self, buffer_size: int = 4096) -> DnetDeviceProperties:
        """Synchronous version of async_get_own_properties."""
        del buffer_size  # Unused
        return self._own_props

    def set_is_busy(self, is_busy: bool) -> None:
        """Set the busy status of the service."""
        self._own_props.is_busy = is_busy
