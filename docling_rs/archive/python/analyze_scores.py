#!/usr/bin/env python3
"""Analyze LLM score changes from N=1021 to N=1379"""

# N=1021 baseline scores (from reports)
baseline = {
    # Verification
    "csv": 100, "html": 98, "markdown": 97, "xlsx": 98, "asciidoc": 97,
    "docx": 100, "pptx": 98, "webvtt": 100, "jats": 98,
    # Mode3 - Archives
    "zip": 90, "tar": 84, "7z": 85, "rar": 85,
    # Mode3 - Email
    "eml": 92, "mbox": 95, "vcf": 90,
    # Mode3 - Ebooks
    "epub": 79, "fb2": 87, "mobi": 85,
    # Mode3 - OpenDocument
    "odt": 75, "ods": 85, "odp": 77,
    # Mode3 - Specialized
    "ics": 92, "ipynb": 92,
    # Mode3 - GPS/GIS (VIOLATED)
    "gpx": 87, "kml": 90, "kmz": 93,
    # Mode3 - Images (VIOLATED)
    "bmp": 88, "gif": 88, "heif": 82, "avif": 82, "svg": 85, "dicom": 87,
    # Mode3 - CAD (VIOLATED)
    "stl": 85, "obj": 90, "gltf": 85, "glb": 95, "dxf": 63,
}

# N=1379 current scores (from test run - corrected)
current = {
    "csv": 100, "html": 100, "markdown": 97, "xlsx": 98, "asciidoc": 98,
    "docx": 100, "pptx": 99, "webvtt": 100, "jats": 92,
    "zip": 85, "tar": 85, "7z": 85, "rar": 85,
    "eml": 87, "mbox": 95, "vcf": 93,
    "epub": 84, "fb2": 83, "mobi": 87,
    "odt": 84, "ods": 85, "odp": 82,
    "ics": 92, "ipynb": 87,
    "gpx": 89, "kml": 93, "kmz": 90,
    "bmp": 86, "gif": 87, "heif": 83, "avif": 83, "svg": 83, "dicom": 92,
    "stl": 83, "obj": 95, "gltf": 85, "glb": 92, "dxf": 82,
}

# Architectural violations fixed (N=1367-1378)
arch_violated = ["gpx", "kml", "kmz", "bmp", "gif", "heif", "avif", "svg", "dicom",
                 "stl", "obj", "gltf", "glb", "dxf"]

print("=" * 80)
print("LLM Score Analysis: N=1021 → N=1379")
print("=" * 80)
print()

# Calculate changes
improvements = {}
regressions = {}
unchanged = {}

for fmt in baseline:
    old = baseline[fmt]
    new = current[fmt]
    diff = new - old

    if diff > 0:
        improvements[fmt] = (old, new, diff)
    elif diff < 0:
        regressions[fmt] = (old, new, diff)
    else:
        unchanged[fmt] = old

# Print improvements
print("IMPROVEMENTS (formats that got better)")
print("-" * 80)
if improvements:
    for fmt in sorted(improvements.items(), key=lambda x: x[1][2], reverse=True):
        name, (old, new, diff) = fmt
        marker = "🔧" if name in arch_violated else "  "
        print(f"{marker} {name.upper():10s}: {old:3d}% → {new:3d}% (+{diff:2d}%)")
else:
    print("None")
print()

# Print regressions
print("REGRESSIONS (formats that got worse)")
print("-" * 80)
if regressions:
    for fmt in sorted(regressions.items(), key=lambda x: x[1][2]):
        name, (old, new, diff) = fmt
        marker = "🔧" if name in arch_violated else "  "
        print(f"{marker} {name.upper():10s}: {old:3d}% → {new:3d}% ({diff:2d}%)")
else:
    print("None")
print()

# Architectural violation analysis
print("ARCHITECTURAL VIOLATION FIXES (🔧 marked above)")
print("-" * 80)
arch_improved = [f for f in arch_violated if f in improvements]
arch_regressed = [f for f in arch_violated if f in regressions]
arch_unchanged = [f for f in arch_violated if f in unchanged]

print(f"Total formats with violations fixed: {len(arch_violated)}")
print(f"  Improved: {len(arch_improved)} ({', '.join(arch_improved) if arch_improved else 'none'})")
print(f"  Regressed: {len(arch_regressed)} ({', '.join(arch_regressed) if arch_regressed else 'none'})")
print(f"  Unchanged: {len(arch_unchanged)} ({', '.join(arch_unchanged) if arch_unchanged else 'none'})")
print()

if arch_improved:
    total_improvement = sum(improvements[f][2] for f in arch_improved)
    print(f"Total improvement from arch fixes: +{total_improvement} percentage points")
    print(f"Average improvement: +{total_improvement / len(arch_improved):.1f} points")
print()

# Summary statistics
print("SUMMARY STATISTICS")
print("-" * 80)
print(f"Total formats tested: {len(baseline)}")
print(f"Improved:   {len(improvements):2d} ({100 * len(improvements) / len(baseline):5.1f}%)")
print(f"Regressed:  {len(regressions):2d} ({100 * len(regressions) / len(baseline):5.1f}%)")
print(f"Unchanged:  {len(unchanged):2d} ({100 * len(unchanged) / len(baseline):5.1f}%)")
print()

total_improvement_pts = sum(diff for _, _, diff in improvements.values())
total_regression_pts = sum(diff for _, _, diff in regressions.values())
net_change = total_improvement_pts + total_regression_pts

print(f"Total improvement: +{total_improvement_pts} percentage points")
print(f"Total regression:  {total_regression_pts} percentage points")
print(f"Net change:        {net_change:+d} percentage points")
print()

# Verdict
print("VERDICT")
print("-" * 80)
if len(arch_improved) >= len(arch_violated) // 2:
    print("✅ ARCHITECTURAL FIXES SUCCESSFUL")
    print(f"   - {len(arch_improved)}/{len(arch_violated)} violated formats improved")
    print(f"   - Net improvement: {total_improvement_pts - abs(total_regression_pts):+d} points")
elif len(improvements) > len(regressions):
    print("⚠️  MIXED RESULTS (More improvements than regressions)")
    print(f"   - Arch fixes helped {len(arch_improved)}/{len(arch_violated)} formats")
    print(f"   - Overall: {len(improvements)} improved, {len(regressions)} regressed")
else:
    print("❌ UNEXPECTED RESULTS")
    print(f"   - Arch fixes only helped {len(arch_improved)}/{len(arch_violated)} formats")
    print(f"   - Overall: {len(improvements)} improved, {len(regressions)} regressed")
print()

# Pass rate comparison
old_pass_count = sum(1 for v in baseline.values() if v >= 95)
new_pass_count = sum(1 for v in current.values() if v >= 95)

print("PASS RATE (≥95%)")
print("-" * 80)
print(f"N=1021: {old_pass_count}/{len(baseline)} ({100 * old_pass_count / len(baseline):.1f}%)")
print(f"N=1379: {new_pass_count}/{len(current)} ({100 * new_pass_count / len(current):.1f}%)")
print(f"Change: {new_pass_count - old_pass_count:+d} formats")
