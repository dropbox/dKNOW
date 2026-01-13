# CRITICAL FIX - Write Results Incrementally

**User**: "I want outputs to be written to an append file as we get them! that's much safer!!!"

**ABSOLUTELY CORRECT** - Current design is terrible:
- Collects ALL results in memory
- Writes at end (line 493-536)
- If crashes after 39 hours: **LOSE EVERYTHING**

---

## THE FIX - Incremental Write

**Edit**: `lib/validate_all_images.py`

### Change 1: Open Files in Append Mode at Start

**After line 391** (after defining file paths):

```python
# Open CSV files in write mode and write headers
csv_file = open(results_csv, 'w', newline='')
csv_writer = csv.DictWriter(csv_file, fieldnames=[
    'index', 'pdf', 'status', 'match', 'total_pages', 'compared_pages',
    'ssim_mean', 'ssim_min', 'ssim_max', 'comparison_errors',
    'category', 'pages', 'error'
])
csv_writer.writeheader()

per_page_file = open(per_page_csv, 'w', newline='')
per_page_writer = csv.DictWriter(per_page_file, fieldnames=[
    'pdf', 'page', 'status', 'match', 'ssim',
    'upstream_md5', 'our_md5', 'md5_match',
    'upstream_width', 'upstream_height',
    'our_width', 'our_height',
    'upstream_bytes', 'our_bytes', 'error'
])
per_page_writer.writeheader()
```

### Change 2: Write After Each PDF

**Inside loop** (after line 427 or 456 where result is received):

```python
result = validator.validate_image_rendering(pdf_info)
result['index'] = idx
results.append(result)

# WRITE IMMEDIATELY - don't wait for end
csv_writer.writerow({
    'index': result.get('index', 0),
    'pdf': result['pdf'],
    'status': result.get('status', 'unknown'),
    'match': result['match'],
    'total_pages': result.get('total_pages', ''),
    'compared_pages': result.get('compared_pages', ''),
    'ssim_mean': f"{result['ssim_mean']:.6f}" if 'ssim_mean' in result else '',
    'ssim_min': f"{result['ssim_min']:.6f}" if 'ssim_min' in result else '',
    'ssim_max': f"{result['ssim_max']:.6f}" if 'ssim_max' in result else '',
    'comparison_errors': result.get('comparison_errors', ''),
    'category': result.get('category', ''),
    'pages': result.get('pages', ''),
    'error': result.get('error', '')
})
csv_file.flush()  # Force write to disk

# Write per-page results
for page_data in result.get('pages_data', []):
    per_page_writer.writerow({
        'pdf': result['pdf'],
        'page': page_data.get('page', ''),
        'status': page_data.get('status', ''),
        'match': page_data.get('match', ''),
        'ssim': f"{page_data['ssim']:.6f}" if 'ssim' in page_data else '',
        'upstream_md5': page_data.get('upstream_md5', ''),
        'our_md5': page_data.get('our_md5', ''),
        'md5_match': page_data.get('md5_match', ''),
        'upstream_width': page_data.get('upstream_width', ''),
        'upstream_height': page_data.get('upstream_height', ''),
        'our_width': page_data.get('our_width', ''),
        'our_height': page_data.get('our_height', ''),
        'upstream_bytes': page_data.get('upstream_bytes', ''),
        'our_bytes': page_data.get('our_bytes', ''),
        'error': page_data.get('error', '')
    })
per_page_file.flush()  # Force write to disk
```

### Change 3: Close Files at End

**Replace lines 493-536** with:

```python
# Close CSV files
csv_file.close()
per_page_file.close()

# Save final JSON summary (this can wait for end)
with open(results_file, 'w') as f:
    json.dump({...}, f, indent=2)
```

---

## Benefits

**Incremental write**:
- ✅ Can monitor progress (tail -f CSV)
- ✅ If crashes at 39h, have 39h of data
- ✅ Can verify correctness early
- ✅ Can restart with --continue-from N

**Current batch write**:
- ❌ No progress visibility
- ❌ Crash = lose all work
- ❌ Can't verify until end
- ❌ Must restart from beginning

---

## WORKER ORDER

**STOP CURRENT PROCESS** (already did)

**FIX validate_all_images.py**:
1. Open CSV files at start
2. Write headers immediately
3. Write each PDF result as it completes
4. Flush after each write
5. Close at end

**RE-RUN**: Safer incremental-write version

**TIME TO FIX**: 30 minutes

**BENEFIT**: Can monitor progress, crash-safe, verify early

---

## This Should Have Been Done From Start

You're right - batch writing 40-hour results is stupid.

Fix it now. Make it incremental.
