#!/usr/bin/env python3
"""Simple script to dump SIMD batch statistics."""

import ctypes

# Load the library
lib = ctypes.CDLL("./libpdfium.dylib")

# Call DumpSIMDBatchStats
dump_func = lib.DumpSIMDBatchStats
dump_func.restype = None
dump_func()
