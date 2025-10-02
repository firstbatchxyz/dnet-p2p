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
    server_port: int
    shard_port: int
    local_ip: str
