#!/usr/bin/env python3
"""
End-to-end validation: Compare Rust ML output vs Python docling output
"""
import sys
sys.path.insert(0, '/Users/ayates/docling')  # Use baseline docling

from docling.document_converter import DocumentConverter
import json
import subprocess

def parse_with_python(pdf_path):
    """Parse PDF with Python docling"""
    print(f"\n=== Parsing with Python docling ===")
    converter = DocumentConverter()
    result = converter.convert(pdf_path)
    
    markdown = result.document.export_to_markdown()
    json_doc = result.document.export_to_dict()
    
    print(f"  Pages: {len(result.document.pages)}")
    print(f"  Markdown length: {len(markdown)} chars")
    print(f"  DocItems: {len(json_doc.get('content', []))}")
    
    return {
        'markdown': markdown,
        'json': json_doc,
        'pages': len(result.document.pages)
    }

def parse_with_rust(pdf_path):
    """Parse PDF with Rust ML backend"""
    print(f"\n=== Parsing with Rust ML ===")
    
    # Create small Rust program to parse PDF
    rust_code = f'''
use docling_backend::pdf::PdfBackend;
use docling_backend::traits::{{BackendOptions, DocumentBackend}};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let backend = PdfBackend::new()?;
    let data = fs::read("{pdf_path}")?;
    let options = BackendOptions::default();
    
    let doc = backend.parse_bytes(&data, &options)?;
    
    // Output as JSON for comparison
    let output = serde_json::json!({{
        "markdown": doc.markdown,
        "pages": doc.metadata.num_pages,
        "characters": doc.metadata.num_characters,
        "has_content_blocks": doc.content_blocks.is_some(),
        "num_doc_items": doc.content_blocks.as_ref().map(|v| v.len()).unwrap_or(0),
        "content_blocks": doc.content_blocks,
    }});
    
    println!("{{}}", serde_json::to_string_pretty(&output)?);
    Ok(())
}}
'''
    
    # Write, compile, run
    with open('/tmp/test_rust_ml.rs', 'w') as f:
        f.write(rust_code)
    
    # Compile
    subprocess.run([
        'rustc', '--edition', '2021',
        '-L', 'target/release/deps',
        '--extern', 'docling_backend=target/release/libdocling_backend.rlib',
        '--extern', 'docling_core=target/release/libdocling_core.rlib',
        '--extern', 'serde_json=target/release/libserde_json.rlib',
        '/tmp/test_rust_ml.rs', '-o', '/tmp/test_rust_ml'
    ], check=True)
    
    # Run
    result = subprocess.run(['/tmp/test_rust_ml'], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"  ERROR: {result.stderr}")
        return None
    
    output = json.loads(result.stdout)
    print(f"  Pages: {output['pages']}")
    print(f"  Markdown length: {len(output['markdown'])} chars")
    print(f"  DocItems: {output['num_doc_items']}")
    print(f"  Has content_blocks: {output['has_content_blocks']}")
    
    return output

def compare_outputs(python_out, rust_out):
    """Compare Python vs Rust outputs"""
    print(f"\n=== Comparison ===")
    
    py_md_len = len(python_out['markdown'])
    rust_md_len = len(rust_out['markdown'])
    
    print(f"Markdown length:")
    print(f"  Python: {py_md_len} chars")
    print(f"  Rust:   {rust_md_len} chars")
    print(f"  Diff:   {rust_md_len - py_md_len} ({((rust_md_len/py_md_len - 1) * 100):.1f}%)")
    
    py_items = len(python_out['json'].get('content', []))
    rust_items = rust_out['num_doc_items']
    
    print(f"\nDocItems:")
    print(f"  Python: {py_items}")
    print(f"  Rust:   {rust_items}")
    print(f"  Diff:   {rust_items - py_items}")
    
    # Character-level comparison
    if python_out['markdown'] == rust_out['markdown']:
        print(f"\n✅ PERFECT MATCH - Outputs identical!")
        return True
    else:
        # Show first difference
        for i, (p, r) in enumerate(zip(python_out['markdown'], rust_out['markdown'])):
            if p != r:
                print(f"\n❌ First diff at char {i}:")
                print(f"  Python: '{python_out['markdown'][max(0,i-20):i+20]}'")
                print(f"  Rust:   '{rust_out['markdown'][max(0,i-20):i+20]}'")
                break
        return False

if __name__ == '__main__':
    pdf_path = 'test-corpus/pdf/2305.03393v1.pdf'
    
    print(f"Validating: {pdf_path}")
    
    python_out = parse_with_python(pdf_path)
    rust_out = parse_with_rust(pdf_path)
    
    if rust_out is None:
        print("\n❌ RUST ML FAILED TO RUN")
        sys.exit(1)
    
    matches = compare_outputs(python_out, rust_out)
    
    if matches:
        print("\n🎉 VALIDATION PASSED - Rust ML == Python quality")
        sys.exit(0)
    else:
        print("\n⚠️  VALIDATION FAILED - Outputs differ")
        sys.exit(1)
