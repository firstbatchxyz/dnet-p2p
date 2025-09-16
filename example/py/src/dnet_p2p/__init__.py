"""
dnet-p2p: Python FFI wrapper for dnet-p2p library.

This package provides a Python interface to the dnet-p2p C library using ctypes.
See: https://github.com/firstbatchxyz/dnet-p2p
"""

from .core import DnetP2P, DnetP2PError, DnetDeviceProperties


__all__ = ["DnetP2P", "DnetP2PError", "DnetDeviceProperties"]