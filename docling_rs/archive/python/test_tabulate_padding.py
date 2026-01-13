#!/usr/bin/env python3
"""Test to understand tabulate's padding behavior for github format."""

from tabulate import tabulate

def test_column_width(content, header="H"):
    """Test a single column with specific content length."""
    data = [[content]]
    headers = [header]
    result = tabulate(data, headers=headers, tablefmt="github")
    lines = result.split('\n')

    # Parse the header line to get column width
    # Format: | header |
    header_line = lines[0]
    # Remove leading/trailing | and spaces
    col_width = len(header_line) - 2  # Remove the two | characters

    content_len = len(content)
    header_len = len(header)
    max_content = max(content_len, header_len)

    # Calculate padding: total_width = | + spaces + content + spaces + |
    # Column content area = col_width - 2 (for the two |)
    # Padding = (col_width - 2) - max_content
    inner_width = col_width - 2
    padding = inner_width - max_content

    print(f"Content: '{content}' (len={content_len})")
    print(f"Header: '{header}' (len={header_len})")
    print(f"Max: {max_content}")
    print(f"Total col width: {col_width}")
    print(f"Inner width: {inner_width}")
    print(f"Padding: {padding}")
    print(f"Result:\n{result}")
    print("-" * 60)
    return padding

# Test various content lengths
test_cases = [
    "a" * 1,
    "a" * 2,
    "a" * 5,
    "a" * 8,
    "a" * 9,
    "a" * 10,
    "a" * 11,
    "a" * 12,
    "a" * 13,
    "a" * 15,
    "a" * 20,
    "a" * 25,
    "a" * 30,
]

print("=" * 60)
print("Testing tabulate padding behavior with github format")
print("=" * 60)

results = []
for content in test_cases:
    padding = test_column_width(content)
    results.append((len(content), padding))

print("\n" + "=" * 60)
print("SUMMARY")
print("=" * 60)
print(f"{'Content Length':<20} {'Padding':<10}")
print("-" * 30)
for content_len, padding in results:
    print(f"{content_len:<20} {padding:<10}")
