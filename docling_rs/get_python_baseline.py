#!/usr/bin/env python3
import sys
import logging
logging.getLogger().setLevel(logging.CRITICAL)
from docling.document_converter import DocumentConverter
c = DocumentConverter()
r = c.convert(sys.argv[1])
md = r.document.export_to_markdown()
print(md, end='')
