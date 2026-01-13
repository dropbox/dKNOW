#!/usr/bin/env python3
"""
Collect TRUE state from codebase - verify every fact.
No speculation, no estimates - measure everything.
"""

import os
import json
import subprocess
from pathlib import Path
from collections import defaultdict


def count_files_in_dir(directory):
    """Count actual files (not directories)"""
    try:
        result = subprocess.run(
            ["find", directory, "-type", "f"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return len([l for l in result.stdout.strip().split("\n") if l])
    except:
        return 0


def get_example_file(directory, max_files=3):
    """Get actual example file paths"""
    try:
        result = subprocess.run(
            ["find", directory, "-type", "f", "-name", "*.*"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        files = [l for l in result.stdout.strip().split("\n") if l][:max_files]
        return files
    except:
        return []


# 1. FORMATS - actual directories and file counts
print("=" * 70)
print("COLLECTING FORMAT DATA")
print("=" * 70)

formats = {}
test_dir = "test_files_wikimedia"

for item in sorted(os.listdir(test_dir)):
    path = os.path.join(test_dir, item)
    if os.path.isdir(path) and item != "MANIFEST.md":
        count = count_files_in_dir(path)
        examples = get_example_file(path, 2)
        formats[item] = {"count": count, "examples": examples}
        print(f"{item:20s}: {count:5d} files")

print(f"\nTotal formats: {len(formats)}")
print(f"Total files: {sum(f['count'] for f in formats.values())}")

# Save to JSON
with open("true_formats.json", "w") as f:
    json.dump(formats, f, indent=2)
print("\nSaved to true_formats.json")

# 2. PLUGINS - actual YAML files
print("\n" + "=" * 70)
print("COLLECTING PLUGIN DATA")
print("=" * 70)

plugins = []
plugin_dir = "config/plugins"

for yaml_file in sorted(Path(plugin_dir).glob("*.yaml")):
    plugin_name = yaml_file.stem
    plugins.append(plugin_name)
    print(f"  {plugin_name}")

print(f"\nTotal plugins: {len(plugins)}")

with open("true_plugins.json", "w") as f:
    json.dump(plugins, f, indent=2)
print("Saved to true_plugins.json")

# 3. TEST COUNTS - actual test functions
print("\n" + "=" * 70)
print("COLLECTING TEST DATA")
print("=" * 70)

test_files = {
    "smoke_test_comprehensive.rs": 0,
    "smoke_test.rs": 0,
    "standard_test_suite.rs": 0,
}

for test_file in test_files.keys():
    path = f"tests/{test_file}"
    if os.path.exists(path):
        result = subprocess.run(
            ["grep", "-c", r"^fn .*test", path], capture_output=True, text=True
        )
        try:
            count = int(result.stdout.strip())
            test_files[test_file] = count
            print(f"{test_file:40s}: {count} test functions")
        except:
            print(f"{test_file:40s}: 0 test functions")

print(f"\nTotal test functions: {sum(test_files.values())}")

with open("true_tests.json", "w") as f:
    json.dump(test_files, f, indent=2)
print("Saved to true_tests.json")

print("\n" + "=" * 70)
print("DATA COLLECTION COMPLETE")
print("=" * 70)
print("Next: Analyze this data to build accurate reports")
