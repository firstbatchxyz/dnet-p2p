"""
Async wrapper for DnetP2P library.

This module provides async versions of the blocking FFI calls to prevent
event loop blocking and thread starvation in asyncio applications.
"""

import asyncio
from typing import Optional, Literal

from .core import DnetP2P, DnetDeviceProperties


class AsyncDnetP2P(DnetP2P):
    """
    Async wrapper for the DnetP2P library.

    All blocking FFI calls are wrapped with asyncio.to_thread() to prevent
    blocking the event loop. This prevents thread starvation when the Rust
    library's background threads need CPU time.

    Usage:
        async_discovery = AsyncDnetP2P("lib/dnet-p2p/lib")
        async_discovery.create_instance("my-instance", 8080, 58080)
        await async_discovery.async_start()

        # Non-blocking discovery calls
        devices = await async_discovery.async_get_properties()
        own_props = await async_discovery.async_get_own_properties()

        await async_discovery.async_stop()
    """

    def __init__(self, library_dir: str = "lib"):
        """Initialize the async DnetP2P wrapper.

        Args:
            library_dir: Path to the directory containing the shared library.
        """
        super().__init__(library_dir)

    async def async_start(
        self,
        loglevel: Optional[Literal["info", "debug", "trace", "warn", "error"]] = None,
    ) -> None:
        """
        Start the dnet service asynchronously.

        This wraps the blocking FFI call in a thread executor to prevent
        blocking the event loop.

        Args:
            loglevel: Optional log level for dnet `RUST_LOG`.

        Raises:
            DnetP2PError: If service is not created or start fails
        """
        loop = asyncio.get_running_loop()

        # Run the blocking start() call in a thread executor
        await loop.run_in_executor(None, self.start, loglevel)

    async def async_stop(self) -> int:
        """
        Stop the dnet service asynchronously.

        Returns:
            int: Status code from the stop operation

        Raises:
            DnetP2PError: If service is not running
        """
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, self.stop)

    async def async_get_properties(
        self, buffer_size: int = 4096
    ) -> dict[str, DnetDeviceProperties]:
        """
        Get the properties of discovered peers asynchronously.

        This wraps the blocking FFI call in a thread executor to prevent
        blocking the event loop and potential thread starvation.

        Args:
            buffer_size: Size of the buffer to allocate for properties data.

        Returns:
            dict[str, DnetDeviceProperties]: A dictionary mapping service names
                                             to their properties.

        Raises:
            DnetP2PError: If no instance is created or if the operation fails
        """
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, self.get_properties, buffer_size)

    async def async_get_own_properties(
        self, buffer_size: int = 4096
    ) -> DnetDeviceProperties:
        """
        Get the service's own properties asynchronously.

        This wraps the blocking FFI call in a thread executor to prevent
        blocking the event loop and potential thread starvation.

        Args:
            buffer_size: Size of the buffer to allocate for properties data.

        Returns:
            DnetDeviceProperties: The service's own properties.

        Raises:
            DnetP2PError: If no instance is created or if the operation fails
        """
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, self.get_own_properties, buffer_size)

    async def async_set_is_busy(self, is_busy: bool) -> None:
        """
        Set the busy status of the service asynchronously.

        Args:
            is_busy: True if the service is busy, False otherwise

        Raises:
            DnetP2PError: If no instance is created
        """
        loop = asyncio.get_running_loop()
        await loop.run_in_executor(None, self.set_is_busy, is_busy)

    async def async_free_instance(self) -> None:
        """
        Free the dnet instance and clean up resources asynchronously.
        """
        loop = asyncio.get_running_loop()
        await loop.run_in_executor(None, self.free_instance)

