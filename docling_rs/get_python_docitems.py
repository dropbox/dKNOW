#!/usr/bin/env python3
"""
Get DocItems from Python docling in JSON format for comparison with Rust.
Usage: python get_python_docitems.py <pdf_file>
"""
import sys
import json
import logging
# Disable ALL logging (including INFO)
logging.basicConfig(level=logging.CRITICAL)
for name in logging.Logger.manager.loggerDict:
    logging.getLogger(name).setLevel(logging.CRITICAL)

from docling.document_converter import DocumentConverter

# Parse PDF
converter = DocumentConverter()
result = converter.convert(sys.argv[1])

# Extract DocItems (the structured content)
doc_items = []
for item in result.document.texts:
    doc_item = {
        "text": item.text,
        "label": item.label.name if hasattr(item.label, 'name') else str(item.label),
        "prov": []
    }

    # Add provenance (bbox) if available
    if hasattr(item, 'prov') and item.prov:
        for prov in item.prov:
            if hasattr(prov, 'bbox'):
                bbox = prov.bbox
                doc_item["prov"].append({
                    "page": prov.page if hasattr(prov, 'page') else 0,
                    "bbox": {
                        "l": bbox.l,
                        "t": bbox.t,
                        "r": bbox.r,
                        "b": bbox.b
                    }
                })

    doc_items.append(doc_item)

# Output as JSON
print(json.dumps(doc_items, indent=2))
