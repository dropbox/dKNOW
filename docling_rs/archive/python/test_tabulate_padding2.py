#!/usr/bin/env python3
"""Test tabulate padding with realistic table data."""

from tabulate import tabulate

# Test with realistic data like what we see in PDF tables
test_data = [
    # Simple test
    [["A", "BB", "CCC"], ["1", "22", "333"]],

    # Medium width columns
    [["Name", "Age", "City"], ["Alice", "25", "New York"]],

    # Varied widths
    [["ID", "Description", "Val"], ["1", "Short text", "10"], ["2", "A much longer description here", "20"]],

    # Very wide columns
    [["A", "B" * 15, "C"], ["x", "y", "z"]],
]

for i, (headers, *rows) in enumerate(test_data):
    print(f"\n{'='*70}")
    print(f"Test Case {i+1}")
    print(f"{'='*70}")

    result = tabulate(rows, headers=headers, tablefmt="github")
    print(result)
    print()

    # Analyze each column
    lines = result.split('\n')
    header_line = lines[0]
    sep_line = lines[1]

    # Parse column widths from separator line
    # Format: |---|---|---|
    parts = sep_line.split('|')
    col_widths = [len(p) for p in parts[1:-1]]  # Skip empty first/last

    print("Column Analysis:")
    for j, header in enumerate(headers):
        col_width = col_widths[j]

        # Get max content width for this column
        max_content = len(header)
        for row in rows:
            max_content = max(max_content, len(row[j]))

        # Calculate padding
        # col_width includes the spaces on both sides but not the |
        padding_total = col_width - max_content

        print(f"  Col {j}: header='{header}' (len={len(header)}), "
              f"max_content={max_content}, col_width={col_width}, "
              f"padding={padding_total}")
