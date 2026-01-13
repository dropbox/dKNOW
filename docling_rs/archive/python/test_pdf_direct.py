#!/usr/bin/env python3
# Just call the Rust backend via Python to see if parse_bytes works

import sys
sys.path.insert(0, 'target/release')

import subprocess

result = subprocess.run([
    'python3', '-c',
    '''
import sys
import ctypes
# This won't work easily, let me just use cargo test approach
print("Using cargo test approach instead")
'''
], capture_output=True, text=True)

print(result.stdout)
