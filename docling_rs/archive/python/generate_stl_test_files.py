#!/usr/bin/env python3
"""
Generate diverse STL test files for docling_rs testing.

STL (STereoLithography) is a 3D mesh format.
- ASCII format: Human-readable text
- Binary format: More compact binary representation

This script generates simple ASCII STL files for testing.
"""

import os
import math

def write_stl_triangle(f, v1, v2, v3):
    """Write a single triangle to STL file."""
    # Calculate normal vector (cross product)
    # For simplicity, we'll use a simple normal (not calculated properly)
    # Real STL files should have proper normals
    normal = [0.0, 0.0, 1.0]

    f.write(f"  facet normal {normal[0]:.6e} {normal[1]:.6e} {normal[2]:.6e}\n")
    f.write("    outer loop\n")
    f.write(f"      vertex {v1[0]:.6e} {v1[1]:.6e} {v1[2]:.6e}\n")
    f.write(f"      vertex {v2[0]:.6e} {v2[1]:.6e} {v2[2]:.6e}\n")
    f.write(f"      vertex {v3[0]:.6e} {v3[1]:.6e} {v3[2]:.6e}\n")
    f.write("    endloop\n")
    f.write("  endfacet\n")

def generate_cube(filename, size=10.0):
    """Generate a simple cube STL file."""
    with open(filename, 'w') as f:
        f.write(f"solid cube_{size}\n")

        s = size / 2
        # 8 vertices of a cube centered at origin
        vertices = [
            (-s, -s, -s),  # 0
            ( s, -s, -s),  # 1
            ( s,  s, -s),  # 2
            (-s,  s, -s),  # 3
            (-s, -s,  s),  # 4
            ( s, -s,  s),  # 5
            ( s,  s,  s),  # 6
            (-s,  s,  s),  # 7
        ]

        # 12 triangles (2 per face)
        faces = [
            # Bottom (z = -s)
            (0, 1, 2), (0, 2, 3),
            # Top (z = s)
            (4, 6, 5), (4, 7, 6),
            # Front (y = -s)
            (0, 5, 1), (0, 4, 5),
            # Back (y = s)
            (3, 2, 6), (3, 6, 7),
            # Left (x = -s)
            (0, 3, 7), (0, 7, 4),
            # Right (x = s)
            (1, 5, 6), (1, 6, 2),
        ]

        for face in faces:
            v1, v2, v3 = vertices[face[0]], vertices[face[1]], vertices[face[2]]
            write_stl_triangle(f, v1, v2, v3)

        f.write(f"endsolid cube_{size}\n")

    print(f"Created: {filename} (simple cube, 12 triangles)")

def generate_pyramid(filename, base_size=10.0, height=15.0):
    """Generate a pyramid STL file."""
    with open(filename, 'w') as f:
        f.write("solid pyramid\n")

        s = base_size / 2
        h = height

        # 5 vertices: 4 base corners + 1 apex
        vertices = [
            (-s, -s, 0),  # 0: base corner
            ( s, -s, 0),  # 1: base corner
            ( s,  s, 0),  # 2: base corner
            (-s,  s, 0),  # 3: base corner
            ( 0,  0, h),  # 4: apex
        ]

        # 6 triangles (2 for base + 4 for sides)
        faces = [
            # Base (z = 0)
            (0, 2, 1), (0, 3, 2),
            # Sides
            (0, 1, 4),  # Front
            (1, 2, 4),  # Right
            (2, 3, 4),  # Back
            (3, 0, 4),  # Left
        ]

        for face in faces:
            v1, v2, v3 = vertices[face[0]], vertices[face[1]], vertices[face[2]]
            write_stl_triangle(f, v1, v2, v3)

        f.write("endsolid pyramid\n")

    print(f"Created: {filename} (pyramid, 6 triangles)")

def generate_complex_shape(filename):
    """Generate a more complex multi-part shape."""
    with open(filename, 'w') as f:
        f.write("solid complex_shape\n")

        # Generate multiple cubes at different positions
        cube_positions = [
            (0, 0, 0, 5),      # center, size 5
            (10, 0, 0, 3),     # right, size 3
            (0, 10, 0, 4),     # back, size 4
            (-10, 0, 5, 6),    # left-up, size 6
        ]

        triangle_count = 0
        for cx, cy, cz, size in cube_positions:
            s = size / 2
            vertices = [
                (cx-s, cy-s, cz-s), (cx+s, cy-s, cz-s),
                (cx+s, cy+s, cz-s), (cx-s, cy+s, cz-s),
                (cx-s, cy-s, cz+s), (cx+s, cy-s, cz+s),
                (cx+s, cy+s, cz+s), (cx-s, cy+s, cz+s),
            ]

            faces = [
                (0, 1, 2), (0, 2, 3), (4, 6, 5), (4, 7, 6),
                (0, 5, 1), (0, 4, 5), (3, 2, 6), (3, 6, 7),
                (0, 3, 7), (0, 7, 4), (1, 5, 6), (1, 6, 2),
            ]

            for face in faces:
                v1, v2, v3 = vertices[face[0]], vertices[face[1]], vertices[face[2]]
                write_stl_triangle(f, v1, v2, v3)
                triangle_count += 1

        f.write("endsolid complex_shape\n")

    print(f"Created: {filename} (complex multi-part, {triangle_count} triangles)")

def generate_large_mesh(filename, subdivisions=50):
    """Generate a large mesh (stress test)."""
    with open(filename, 'w') as f:
        f.write("solid large_mesh\n")

        triangle_count = 0
        # Generate a grid of triangles
        for i in range(subdivisions):
            for j in range(subdivisions):
                # Two triangles per grid square
                x1, y1 = i, j
                x2, y2 = i+1, j
                x3, y3 = i+1, j+1
                x4, y4 = i, j+1

                # Z varies sinusoidally
                z1 = math.sin(x1/5) * math.cos(y1/5) * 2
                z2 = math.sin(x2/5) * math.cos(y2/5) * 2
                z3 = math.sin(x3/5) * math.cos(y3/5) * 2
                z4 = math.sin(x4/5) * math.cos(y4/5) * 2

                # Triangle 1
                write_stl_triangle(f, (x1, y1, z1), (x2, y2, z2), (x3, y3, z3))
                triangle_count += 1

                # Triangle 2
                write_stl_triangle(f, (x1, y1, z1), (x3, y3, z3), (x4, y4, z4))
                triangle_count += 1

        f.write("endsolid large_mesh\n")

    print(f"Created: {filename} (large mesh, {triangle_count} triangles)")

def generate_minimal(filename):
    """Generate a minimal STL file (single triangle)."""
    with open(filename, 'w') as f:
        f.write("solid minimal\n")
        write_stl_triangle(f, (0, 0, 0), (1, 0, 0), (0, 1, 0))
        f.write("endsolid minimal\n")

    print(f"Created: {filename} (minimal, 1 triangle)")

def main():
    """Generate all test STL files."""
    output_dir = "test-corpus/cad/stl"
    os.makedirs(output_dir, exist_ok=True)

    print("Generating STL test files...\n")

    # 1. Simple cube (geometric primitive)
    generate_cube(f"{output_dir}/simple_cube.stl", size=10.0)

    # 2. Pyramid (basic shape)
    generate_pyramid(f"{output_dir}/pyramid.stl", base_size=10.0, height=15.0)

    # 3. Complex multi-part shape
    generate_complex_shape(f"{output_dir}/complex_shape.stl")

    # 4. Large mesh (stress test)
    generate_large_mesh(f"{output_dir}/large_mesh.stl", subdivisions=50)

    # 5. Minimal (edge case, single triangle)
    generate_minimal(f"{output_dir}/minimal_triangle.stl")

    print("\n✅ All 5 STL test files generated successfully!")
    print(f"\nFiles created in: {output_dir}/")
    print("  - simple_cube.stl (12 triangles)")
    print("  - pyramid.stl (6 triangles)")
    print("  - complex_shape.stl (~48 triangles)")
    print("  - large_mesh.stl (~5000 triangles)")
    print("  - minimal_triangle.stl (1 triangle)")

if __name__ == "__main__":
    main()
