"""
Python FFI wrapper for dnet-p2p library.
This module provides a Python interface to the dnet-p2p C library using ctypes.
"""

import ctypes
import json
import os
import platform

from pydantic import BaseModel


class DnetServiceMemoryProperties(BaseModel):
    """Model representing memory properties of a dnet service."""

    avail: int  # Available memory in bytes
    total: int  # Total memory in bytes
    free: int  # Free memory in bytes


class DnetServiceCPUProperties(BaseModel):
    """Model representing CPU properties of a dnet service."""

    brand: str


class DnetServiceGPUProperties(BaseModel):
    """Model representing GPU properties of a dnet service."""

    name: str
    kind: str


class DnetDeviceProperties(BaseModel):
    """Model representing the properties of a dnet device."""

    mem: DnetServiceMemoryProperties
    cpus: list[DnetServiceCPUProperties]
    gpus: list[DnetServiceGPUProperties]
    address: str
    is_manager: bool
    is_busy: bool
    hostname: str
    instance: str


class DnetP2PError(Exception):
    """Exception raised for dnet-p2p related errors."""


class DnetP2P:
    """
    Python wrapper for the dnet-p2p library.

    This class provides a Pythonic interface to the C library functions
    for creating and managing dnet-p2p instances.

    On error, raises DnetP2PError.
    """

    def __init__(self, library_dir: str = "lib"):
        """
        Initialize the DnetP2P wrapper.

        Args:
            library_path: Optional path to the shared library. If not provided,
                         will attempt to find the library automatically.
        """
        self._lib = self._load_library(library_dir)
        self._setup_function_signatures()
        self._service_ptr = None
        self._handle_ptr = None

    def _load_library(self, library_dir: str) -> ctypes.CDLL:
        """Load the dnet-p2p shared library.

        Args:
            library_dir: Optional path to the directory containing the shared library.
                         If None, will attempt to find the library automatically.

        Returns:
            ctypes.CDLL: Loaded shared library object.

        Raises:
            DnetP2PError: If the library cannot be found or loaded.
        """
        # determine the library name based on the platform
        system = platform.system()
        if system == "Darwin":  # macOS
            lib_name = "libdnet_p2p.dylib"
        elif system == "Linux":
            lib_name = "libdnet_p2p.so"
        elif system == "Windows":
            lib_name = "dnet_p2p.dll"
        else:
            raise DnetP2PError(f"Unsupported platform: {system}")

        # Use the provided library directory
        library_path = os.path.join(library_dir, lib_name)
        if not os.path.exists(library_path):
            raise DnetP2PError(f"Library not found at {library_path}")

        try:
            print(f"Loading dnet-p2p library from {library_path}")
            return ctypes.CDLL(library_path)
        except OSError as e:
            raise DnetP2PError(
                f"Failed to load library from {library_path}: {e}"
            ) from e

    def _setup_function_signatures(self):
        """Set up function signatures for type safety."""
        # dnet_p2p_enable_logs
        self._lib.dnet_p2p_enable_logs.argtypes = []
        self._lib.dnet_p2p_enable_logs.restype = None

        # dnet_p2p_new
        self._lib.dnet_p2p_new.argtypes = [
            ctypes.c_char_p,  # instance
            ctypes.c_char_p,  # hostname
            ctypes.c_char_p,  # address
            ctypes.c_int,  # is_manager
            ctypes.c_int,  # is_passive
        ]
        self._lib.dnet_p2p_new.restype = ctypes.c_void_p

        # dnet_p2p_free
        self._lib.dnet_p2p_free.argtypes = [ctypes.c_void_p]
        self._lib.dnet_p2p_free.restype = None

        # dnet_p2p_start
        self._lib.dnet_p2p_start.argtypes = [ctypes.c_void_p]
        self._lib.dnet_p2p_start.restype = ctypes.c_void_p

        # dnet_p2p_stop
        self._lib.dnet_p2p_stop.argtypes = [
            ctypes.c_void_p,  # service_ptr
            ctypes.c_void_p,  # handle_ptr
        ]
        self._lib.dnet_p2p_stop.restype = ctypes.c_int

        # dnet_p2p_get_properties
        self._lib.dnet_p2p_get_properties.argtypes = [
            ctypes.c_void_p,  # service_ptr
            ctypes.c_void_p,  # buf
            ctypes.c_size_t,  # buf_size
        ]
        self._lib.dnet_p2p_get_properties.restype = ctypes.c_int

        # dnet_p2p_set_is_busy
        self._lib.dnet_p2p_set_is_busy.argtypes = [
            ctypes.c_void_p,  # service_ptr
            ctypes.c_bool,  # is_busy
        ]
        self._lib.dnet_p2p_set_is_busy.restype = None

    def enable_logs(self):
        """
        Enable logging for dnet, respecting the RUST_LOG environment variable.
        """
        self._lib.dnet_p2p_enable_logs()

    def create_instance(
        self,
        instance: str,
        hostname: str,
        address: str,
        is_manager: bool = False,
        is_passive: bool = False,
    ):
        """
        Create a new dnet instance.

        Args:
            instance: Name of the dnet instance
            hostname: Hostname to bind to, e.g. from `gethostname()` system call
            address: Address that the instance has a service on.
            is_manager: If `True`, the instance will run in manager mode,
                       otherwise in worker mode
            is_passive: If `True`, the instance will only monitor (not register to mDNS)

        Raises:
            DnetP2PError: If instance creation fails
        """
        if self._service_ptr is not None:
            raise DnetP2PError("Instance already created. Call free_instance() first.")

        instance_bytes = instance.encode("utf-8")
        hostname_bytes = hostname.encode("utf-8")
        address_bytes = address.encode("utf-8")

        self._service_ptr = self._lib.dnet_p2p_new(
            instance_bytes,
            hostname_bytes,
            address_bytes,
            1 if is_manager else 0,
            1 if is_passive else 0,
        )

        if self._service_ptr is None:
            raise DnetP2PError("Failed to create dnet instance")

    def start(self):
        """
        Start the dnet service.

        Raises:
            DnetP2PError: If service is not created or start fails
        """
        if self._service_ptr is None:
            raise DnetP2PError("No instance created. Call create_instance() first.")

        if self._handle_ptr is not None:
            raise DnetP2PError("Service already started. Call stop() first.")

        self._handle_ptr = self._lib.dnet_p2p_start(self._service_ptr)

        if self._handle_ptr is None:
            raise DnetP2PError("Failed to start dnet service")

    def stop(self):
        """
        Stop the dnet service.

        Returns:
            int: Status code from the stop operation

        Raises:
            DnetP2PError: If service is not running
        """
        if self._service_ptr is None:
            raise DnetP2PError("No instance created.")

        if self._handle_ptr is None:
            raise DnetP2PError("Service not started.")

        result = self._lib.dnet_p2p_stop(self._service_ptr, self._handle_ptr)
        self._handle_ptr = None
        return result

    def free_instance(self):
        """
        Free the dnet instance and clean up resources.
        """
        if self._handle_ptr is not None:
            self.stop()

        if self._service_ptr is not None:
            self._lib.dnet_p2p_free(self._service_ptr)
            self._service_ptr = None

    def is_running(self) -> bool:
        """
        Check if the service is currently running.

        Returns:
            bool: True if the service is running, False otherwise
        """
        return self._handle_ptr is not None

    def is_created(self) -> bool:
        """
        Check if an instance has been created.

        Returns:
            bool: True if an instance exists, False otherwise
        """
        return self._service_ptr is not None

    def get_properties(
        self, buffer_size: int = 2048
    ) -> dict[str, DnetDeviceProperties]:
        """
        Get the properties of the dnet service.

        Args:
            buffer_size: Size of the buffer to allocate for properties data. (default: 2048)

        Returns:
            bytes: The properties data returned by the service

        Raises:
            DnetP2PError: If no instance is created or if the operation fails
            UnicodeDecodeError: If the data cannot be decoded with the specified encoding
            ValidationError: If the properties data does not match the expected format
        """
        if self._service_ptr is None:
            raise DnetP2PError("No instance created.")

        # create a buffer to receive the properties & call
        buffer = ctypes.create_string_buffer(buffer_size)
        result = self._lib.dnet_p2p_get_properties(
            self._service_ptr, buffer, buffer_size
        )

        if result < 0:
            raise DnetP2PError(f"Failed to get properties (error code: {result})")

        # get the buffer up to the first null terminator
        properties_bytes = buffer.raw.rstrip(b"\x00")

        # deserialize & validate
        properties_str = properties_bytes.decode("utf-8")
        properties_json: dict[str, object] = json.loads(properties_str)
        # iterate each key in the JSON and convert to DnetDeviceProperties
        if not isinstance(properties_json, dict):
            raise DnetP2PError("Properties data is not a valid JSON object")
        properties: dict[str, DnetDeviceProperties] = {}
        for key, value in properties_json.items():
            properties[key] = DnetDeviceProperties.model_validate(value)

        return properties

    def set_is_busy(self, is_busy: bool):
        """
        Set the busy status of the service.

        This updates the is_busy property of the service and refreshes mDNS
        if the service is not in passive mode.

        Args:
            is_busy: True if the service is busy, False otherwise

        Raises:
            DnetP2PError: If no instance is created
        """
        if self._service_ptr is None:
            raise DnetP2PError("No instance created.")

        self._lib.dnet_p2p_set_is_busy(self._service_ptr, is_busy)

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - clean up resources."""
        self.free_instance()

    def __del__(self):
        """Destructor - ensure resources are cleaned up."""
        self.free_instance()
