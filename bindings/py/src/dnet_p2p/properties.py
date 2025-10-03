from pydantic import BaseModel


class RawProperties(BaseModel):
    """Model representing the properties of a dnet device."""

    is_manager: bool
    """Indicates that the device is manager (usually the API)."""
    is_busy: bool
    """Indicates that the device is busy (e.g. doing an inference)."""
    instance: str
    """Instance name, expected to be unique per device, but not enforced."""

    host: str
    """Host that this device is bound to, usually 0.0.0.0."""
    server_port: int
    """Port that the HTTP server is bound to."""
    shard_port: int
    """Port that the shard service is bound to."""
    local_ip: str
    """Local IP address of the device, reach via WiFi etc."""
