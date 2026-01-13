#!/usr/bin/env python3
"""
Generate diverse GLTF test files for docling_rs testing.

This script downloads sample GLTF files from the Khronos glTF-Sample-Models
repository and creates a simple programmatic GLTF file.

Test files:
1. simple_triangle.gltf - Simple triangle (programmatically generated)
2. box.gltf - Basic cube from Khronos samples
3. textured_cube.gltf - Cube with texture from Khronos samples
4. duck.gltf - Duck model from Khronos samples (classic test model)
5. scene.gltf - Multi-object scene from Khronos samples
"""

import json
import urllib.request
import os
import base64
import struct

OUTPUT_DIR = "test-corpus/cad/gltf"

def create_simple_triangle():
    """Create a simple GLTF triangle programmatically."""

    # Triangle vertices (positions)
    positions = [
        0.0, 0.0, 0.0,    # vertex 0
        1.0, 0.0, 0.0,    # vertex 1
        0.5, 1.0, 0.0,    # vertex 2
    ]

    # Pack vertices as binary (little-endian floats)
    position_bytes = struct.pack(f'<{len(positions)}f', *positions)
    position_base64 = base64.b64encode(position_bytes).decode('utf-8')

    # Triangle indices
    indices = [0, 1, 2]
    indices_bytes = struct.pack(f'<{len(indices)}H', *indices)  # Unsigned shorts
    indices_base64 = base64.b64encode(indices_bytes).decode('utf-8')

    gltf = {
        "asset": {
            "version": "2.0",
            "generator": "docling_rs test generator"
        },
        "scene": 0,
        "scenes": [
            {
                "name": "Simple Triangle Scene",
                "nodes": [0]
            }
        ],
        "nodes": [
            {
                "name": "Triangle",
                "mesh": 0
            }
        ],
        "meshes": [
            {
                "name": "TriangleMesh",
                "primitives": [
                    {
                        "attributes": {
                            "POSITION": 0
                        },
                        "indices": 1,
                        "mode": 4  # TRIANGLES
                    }
                ]
            }
        ],
        "accessors": [
            {
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,  # FLOAT
                "count": 3,
                "type": "VEC3",
                "min": [0.0, 0.0, 0.0],
                "max": [1.0, 1.0, 0.0]
            },
            {
                "bufferView": 1,
                "byteOffset": 0,
                "componentType": 5123,  # UNSIGNED_SHORT
                "count": 3,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            {
                "buffer": 0,
                "byteOffset": 0,
                "byteLength": len(position_bytes),
                "target": 34962  # ARRAY_BUFFER
            },
            {
                "buffer": 1,
                "byteOffset": 0,
                "byteLength": len(indices_bytes),
                "target": 34963  # ELEMENT_ARRAY_BUFFER
            }
        ],
        "buffers": [
            {
                "byteLength": len(position_bytes),
                "uri": f"data:application/octet-stream;base64,{position_base64}"
            },
            {
                "byteLength": len(indices_bytes),
                "uri": f"data:application/octet-stream;base64,{indices_base64}"
            }
        ]
    }

    output_path = os.path.join(OUTPUT_DIR, "simple_triangle.gltf")
    with open(output_path, 'w') as f:
        json.dump(gltf, f, indent=2)
    print(f"✓ Created: {output_path} ({os.path.getsize(output_path)} bytes)")

def download_khronos_sample(model_name, filename, target_filename):
    """Download a sample GLTF file from Khronos repository."""
    base_url = "https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Models/main/2.0"
    url = f"{base_url}/{model_name}/glTF/{filename}"
    output_path = os.path.join(OUTPUT_DIR, target_filename)

    try:
        print(f"Downloading: {url}")
        urllib.request.urlretrieve(url, output_path)
        file_size = os.path.getsize(output_path)
        print(f"✓ Downloaded: {output_path} ({file_size} bytes)")
        return True
    except Exception as e:
        print(f"✗ Failed to download {url}: {e}")
        return False

def download_khronos_binary(model_name, filename, target_filename):
    """Download a binary GLB file from Khronos repository."""
    base_url = "https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Models/main/2.0"
    url = f"{base_url}/{model_name}/glTF-Binary/{filename}"
    output_path = os.path.join(OUTPUT_DIR, target_filename)

    try:
        print(f"Downloading: {url}")
        urllib.request.urlretrieve(url, output_path)
        file_size = os.path.getsize(output_path)
        print(f"✓ Downloaded: {output_path} ({file_size} bytes)")
        return True
    except Exception as e:
        print(f"✗ Failed to download {url}: {e}")
        return False

def create_simple_cube():
    """Create a simple cube GLTF file programmatically."""
    # Cube vertices (8 vertices, 3 components each)
    positions = [
        # Front face
        -0.5, -0.5,  0.5,  # 0
         0.5, -0.5,  0.5,  # 1
         0.5,  0.5,  0.5,  # 2
        -0.5,  0.5,  0.5,  # 3
        # Back face
        -0.5, -0.5, -0.5,  # 4
         0.5, -0.5, -0.5,  # 5
         0.5,  0.5, -0.5,  # 6
        -0.5,  0.5, -0.5,  # 7
    ]

    # Cube indices (12 triangles = 36 indices)
    indices = [
        # Front
        0, 1, 2,  0, 2, 3,
        # Right
        1, 5, 6,  1, 6, 2,
        # Back
        5, 4, 7,  5, 7, 6,
        # Left
        4, 0, 3,  4, 3, 7,
        # Top
        3, 2, 6,  3, 6, 7,
        # Bottom
        4, 5, 1,  4, 1, 0,
    ]

    position_bytes = struct.pack(f'<{len(positions)}f', *positions)
    position_base64 = base64.b64encode(position_bytes).decode('utf-8')

    indices_bytes = struct.pack(f'<{len(indices)}H', *indices)
    indices_base64 = base64.b64encode(indices_bytes).decode('utf-8')

    gltf = {
        "asset": {
            "version": "2.0",
            "generator": "docling_rs test generator"
        },
        "scene": 0,
        "scenes": [
            {
                "name": "Cube Scene",
                "nodes": [0]
            }
        ],
        "nodes": [
            {
                "name": "Cube",
                "mesh": 0
            }
        ],
        "meshes": [
            {
                "name": "CubeMesh",
                "primitives": [
                    {
                        "attributes": {
                            "POSITION": 0
                        },
                        "indices": 1,
                        "mode": 4
                    }
                ]
            }
        ],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": 8,
                "type": "VEC3",
                "min": [-0.5, -0.5, -0.5],
                "max": [0.5, 0.5, 0.5]
            },
            {
                "bufferView": 1,
                "componentType": 5123,
                "count": 36,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            {
                "buffer": 0,
                "byteLength": len(position_bytes),
                "target": 34962
            },
            {
                "buffer": 1,
                "byteLength": len(indices_bytes),
                "target": 34963
            }
        ],
        "buffers": [
            {
                "byteLength": len(position_bytes),
                "uri": f"data:application/octet-stream;base64,{position_base64}"
            },
            {
                "byteLength": len(indices_bytes),
                "uri": f"data:application/octet-stream;base64,{indices_base64}"
            }
        ]
    }

    output_path = os.path.join(OUTPUT_DIR, "simple_cube.gltf")
    with open(output_path, 'w') as f:
        json.dump(gltf, f, indent=2)
    print(f"✓ Created: {output_path} ({os.path.getsize(output_path)} bytes)")

def main():
    """Generate all GLTF test files."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    print("Generating GLTF test files...")
    print(f"Output directory: {OUTPUT_DIR}\n")

    # 1. Simple triangle (programmatic)
    print("[1/7] Creating simple triangle...")
    create_simple_triangle()
    print()

    # 2. Simple cube (programmatic)
    print("[2/7] Creating simple cube...")
    create_simple_cube()
    print()

    # 3. Box from Khronos samples
    print("[3/7] Downloading Box model...")
    download_khronos_sample("Box", "Box.gltf", "box.gltf")
    print()

    # 4. Duck model (classic test model)
    print("[4/7] Downloading Duck model...")
    download_khronos_sample("Duck", "Duck.gltf", "duck.gltf")
    print()

    # 5. Binary GLB format (Box in GLB)
    print("[5/7] Downloading Box GLB (binary format)...")
    download_khronos_binary("Box", "Box.glb", "box.glb")
    print()

    # 6. Avocado (textured model)
    print("[6/7] Downloading Avocado model...")
    download_khronos_sample("Avocado", "Avocado.gltf", "avocado.gltf")
    print()

    # 7. Triangle (minimal sample)
    print("[7/7] Downloading Triangle model...")
    download_khronos_sample("Triangle", "Triangle.gltf", "triangle.gltf")
    print()

    print("=" * 60)
    print("GLTF test file generation complete!")
    print(f"Files created in: {OUTPUT_DIR}")

    # List all created files
    files = sorted(os.listdir(OUTPUT_DIR))
    print(f"\nTotal files: {len(files)}")
    for filename in files:
        filepath = os.path.join(OUTPUT_DIR, filename)
        size = os.path.getsize(filepath)
        print(f"  - {filename} ({size} bytes)")

if __name__ == "__main__":
    main()
