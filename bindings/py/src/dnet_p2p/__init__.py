"""
dnet-p2p: Python FFI wrapper for dnet-p2p library.

This package provides a Python interface to the dnet-p2p C library using ctypes.
See: https://github.com/firstbatchxyz/dnet-p2p
"""

from .core import DnetP2P, DnetP2PError, DnetDeviceProperties
from .core_async import AsyncDnetP2P
from .manual import load_manual_devices, merge_device_mappings
from .static import StaticDiscovery, load_hostfile
from .thunderbolt import (
    ThunderboltData,
    ThunderboltInstance,
    ThunderboltConnection,
    discover_all_thunderbolt_connections,
    discover_thunderbolt_connection,
)

__all__ = [
    ## core.py
    "DnetP2P",
    "DnetP2PError",
    "DnetDeviceProperties",
    ## core_async.py
    "AsyncDnetP2P",
    ## manual.py
    "load_manual_devices",
    "merge_device_mappings",
    ## static.py
    "StaticDiscovery",
    "load_hostfile",
    ## thunderbolt.py
    "ThunderboltData",
    "ThunderboltInstance",
    "ThunderboltConnection",
    "discover_all_thunderbolt_connections",
    "discover_thunderbolt_connection",
]

