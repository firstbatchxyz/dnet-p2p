from pydantic import BaseModel, Field


class RawProperties(BaseModel):
    """Model representing the properties of a dnet device."""

    is_manager: bool = Field(
        default=False,
        description="Indicates that the device is manager (usually the API).",
    )
    is_busy: bool = Field(
        default=False,
        description="Indicates that the device is busy (e.g. doing an inference).",
    )
    instance: str = Field(
        ...,
        description="Instance name, expected to be unique per device, but not enforced.",
    )
    server_port: int = Field(
        ...,
        description="Port that the HTTP server is bound to.",
    )
    shard_port: int = Field(
        ...,
        description="Port that the shard service is bound to.",
    )
    local_ip: str = Field(
        ...,
        description="Local IP address of the device, reach via WiFi etc.",
    )
