"""
dnet-p2p: Python FFI wrapper for dnet-p2p library.

This package provides a Python interface to the dnet-p2p C library using ctypes.
See: https://github.com/firstbatchxyz/dnet-p2p
"""

from .core import DnetP2P, DnetP2PError, DnetDeviceProperties
from .manual import load_manual_devices, merge_device_mappings
from .thunderbolt import ThunderboltData, ThunderboltInstance

__all__ = [
    ## core.py
    "DnetP2P",
    "DnetP2PError",
    "DnetDeviceProperties",
    ## manual.py
    "load_manual_devices",
    "merge_device_mappings",
    ## thunderbolt.py
    "ThunderboltData",
    "ThunderboltInstance",
]
