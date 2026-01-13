#!/usr/bin/env python3
"""
Generate test DXF files for docling_rs integration tests.

This script creates 5 diverse DXF test files using the dxf Python library (ezdxf).
DXF (Drawing Exchange Format) is a CAD data file format developed by Autodesk.

Requirements:
    pip install ezdxf

Test Files Generated:
    1. simple_drawing.dxf - Simple 2D drawing with lines, circles, and text
    2. floor_plan.dxf - Basic floor plan with rectangles and text labels
    3. mechanical_part.dxf - Mechanical part with circles, arcs, and dimensions
    4. electrical_schematic.dxf - Simple electrical schematic with symbols
    5. 3d_model.dxf - Simple 3D model with polylines and text
"""

import os
import sys
from pathlib import Path

try:
    import ezdxf
except ImportError:
    print("Error: ezdxf library not installed")
    print("Please install it with: pip install ezdxf")
    sys.exit(1)


def create_simple_drawing(output_path: Path):
    """Create a simple 2D drawing with basic entities."""
    doc = ezdxf.new('R2010')
    msp = doc.modelspace()

    # Add a rectangle
    msp.add_lwpolyline([(0, 0), (10, 0), (10, 5), (0, 5), (0, 0)])

    # Add a circle
    msp.add_circle((5, 2.5), radius=1.5)

    # Add lines
    msp.add_line((0, 0), (10, 5))
    msp.add_line((10, 0), (0, 5))

    # Add text
    msp.add_text("Simple Drawing", dxfattribs={'height': 0.5}).set_placement((2, 6))

    doc.saveas(output_path)
    print(f"Created: {output_path}")


def create_floor_plan(output_path: Path):
    """Create a basic floor plan with rooms and labels."""
    doc = ezdxf.new('R2010')
    msp = doc.modelspace()

    # Room 1 - Living Room
    msp.add_lwpolyline([(0, 0), (8, 0), (8, 6), (0, 6), (0, 0)])
    msp.add_text("Living Room", dxfattribs={'height': 0.3}).set_placement((2, 3))

    # Room 2 - Kitchen
    msp.add_lwpolyline([(8, 0), (14, 0), (14, 4), (8, 4), (8, 0)])
    msp.add_text("Kitchen", dxfattribs={'height': 0.3}).set_placement((9, 2))

    # Room 3 - Bedroom
    msp.add_lwpolyline([(8, 4), (14, 4), (14, 10), (8, 10), (8, 4)])
    msp.add_text("Bedroom", dxfattribs={'height': 0.3}).set_placement((9, 7))

    # Doors (as openings)
    msp.add_line((8, 2), (8, 3))  # Door to kitchen
    msp.add_line((8, 6), (8, 7))  # Door to bedroom

    # Title
    msp.add_text("Floor Plan - 2 Bedroom Apartment", dxfattribs={'height': 0.5}).set_placement((2, 11))

    doc.saveas(output_path)
    print(f"Created: {output_path}")


def create_mechanical_part(output_path: Path):
    """Create a mechanical part drawing with dimensions."""
    doc = ezdxf.new('R2010')
    msp = doc.modelspace()

    # Main body (rectangle)
    msp.add_lwpolyline([(0, 0), (8, 0), (8, 4), (0, 4), (0, 0)])

    # Mounting holes (circles)
    msp.add_circle((1, 1), radius=0.25)
    msp.add_circle((7, 1), radius=0.25)
    msp.add_circle((1, 3), radius=0.25)
    msp.add_circle((7, 3), radius=0.25)

    # Center hole
    msp.add_circle((4, 2), radius=0.75)

    # Arc cutout
    msp.add_arc((8, 2), radius=1.5, start_angle=90, end_angle=270)

    # Add dimensions text (simplified)
    msp.add_text("8.00", dxfattribs={'height': 0.2}).set_placement((3.5, -0.5))
    msp.add_text("4.00", dxfattribs={'height': 0.2}).set_placement((-0.8, 1.8))
    msp.add_text("Ø1.50", dxfattribs={'height': 0.2}).set_placement((4.5, 2.5))

    # Title
    msp.add_text("Mechanical Part - Mounting Bracket", dxfattribs={'height': 0.3}).set_placement((1, 5))

    doc.saveas(output_path)
    print(f"Created: {output_path}")


def create_electrical_schematic(output_path: Path):
    """Create a simple electrical schematic."""
    doc = ezdxf.new('R2010')
    msp = doc.modelspace()

    # Power source (circle with +/-)
    msp.add_circle((2, 4), radius=0.5)
    msp.add_text("+", dxfattribs={'height': 0.3}).set_placement((1.8, 4.2))
    msp.add_text("-", dxfattribs={'height': 0.3}).set_placement((1.8, 3.5))

    # Resistor (rectangle)
    msp.add_lwpolyline([(4, 3.5), (6, 3.5), (6, 4.5), (4, 4.5), (4, 3.5)])
    msp.add_text("R1\n100Ω", dxfattribs={'height': 0.2}).set_placement((4.2, 4.7))

    # LED (triangle)
    msp.add_line((8, 3.5), (9, 4))
    msp.add_line((9, 4), (8, 4.5))
    msp.add_line((8, 4.5), (8, 3.5))
    msp.add_text("LED1", dxfattribs={'height': 0.2}).set_placement((8.2, 4.7))

    # Wires (connections)
    msp.add_line((2.5, 4), (4, 4))      # Battery to resistor
    msp.add_line((6, 4), (8, 4))         # Resistor to LED
    msp.add_line((9, 4), (10, 4))        # LED positive
    msp.add_line((10, 4), (10, 2))       # Down to ground
    msp.add_line((10, 2), (2, 2))        # Ground line
    msp.add_line((2, 2), (2, 3.5))       # Back to battery

    # Ground symbol
    msp.add_line((9.5, 2), (10.5, 2))
    msp.add_line((9.7, 1.8), (10.3, 1.8))
    msp.add_line((9.9, 1.6), (10.1, 1.6))

    # Title
    msp.add_text("Electrical Schematic - LED Circuit", dxfattribs={'height': 0.3}).set_placement((2, 6))

    doc.saveas(output_path)
    print(f"Created: {output_path}")


def create_3d_model(output_path: Path):
    """Create a simple 3D model."""
    doc = ezdxf.new('R2010')
    msp = doc.modelspace()

    # 3D box (using 3D polylines)
    # Bottom face
    msp.add_line((0, 0, 0), (4, 0, 0))
    msp.add_line((4, 0, 0), (4, 3, 0))
    msp.add_line((4, 3, 0), (0, 3, 0))
    msp.add_line((0, 3, 0), (0, 0, 0))

    # Top face
    msp.add_line((0, 0, 2), (4, 0, 2))
    msp.add_line((4, 0, 2), (4, 3, 2))
    msp.add_line((4, 3, 2), (0, 3, 2))
    msp.add_line((0, 3, 2), (0, 0, 2))

    # Vertical edges
    msp.add_line((0, 0, 0), (0, 0, 2))
    msp.add_line((4, 0, 0), (4, 0, 2))
    msp.add_line((4, 3, 0), (4, 3, 2))
    msp.add_line((0, 3, 0), (0, 3, 2))

    # Add 3D points to mark corners
    msp.add_point((0, 0, 0))
    msp.add_point((4, 3, 2))

    # Text annotation
    msp.add_text("3D Box Model", dxfattribs={'height': 0.3}).set_placement((1, 4, 0))
    msp.add_text("Dimensions: 4x3x2", dxfattribs={'height': 0.2}).set_placement((1, 3.5, 0))

    doc.saveas(output_path)
    print(f"Created: {output_path}")


def main():
    """Generate all test DXF files."""
    # Determine output directory
    script_dir = Path(__file__).parent
    repo_root = script_dir.parent
    output_dir = repo_root / "test-corpus" / "cad" / "dxf"

    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)
    print(f"Output directory: {output_dir}")

    # Generate test files
    create_simple_drawing(output_dir / "simple_drawing.dxf")
    create_floor_plan(output_dir / "floor_plan.dxf")
    create_mechanical_part(output_dir / "mechanical_part.dxf")
    create_electrical_schematic(output_dir / "electrical_schematic.dxf")
    create_3d_model(output_dir / "3d_model.dxf")

    print(f"\nSuccessfully generated 5 DXF test files in: {output_dir}")
    print("\nTest files:")
    print("  1. simple_drawing.dxf - Basic 2D shapes and text")
    print("  2. floor_plan.dxf - Architectural floor plan")
    print("  3. mechanical_part.dxf - Mechanical part with dimensions")
    print("  4. electrical_schematic.dxf - Electrical circuit diagram")
    print("  5. 3d_model.dxf - Simple 3D box model")


if __name__ == "__main__":
    main()
