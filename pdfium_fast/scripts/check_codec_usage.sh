#!/bin/bash
# Analyze which image codecs are used in the PDF corpus

echo "Analyzing codec usage in PDF corpus..."
echo "Scanning 462 PDFs across all categories..."

rm -f /tmp/codec_usage.txt

for pdf in integration_tests/pdfs/*/*.pdf; do
    # Extract PDF streams and filters
    strings "$pdf" | grep -E "/(FlateDecode|DCTDecode|JBIG2Decode|JPXDecode|CCITTFaxDecode)" >> /tmp/codec_usage.txt
done

echo ""
echo "=== Codec Usage Report ==="
echo ""
echo "JPEG (DCTDecode - libjpeg-turbo):"
grep -c "DCTDecode" /tmp/codec_usage.txt || echo "0"
echo ""
echo "JBIG2 (Monochrome compression):"
grep -c "JBIG2Decode" /tmp/codec_usage.txt || echo "0"
echo ""
echo "JPEG2000 (JPXDecode):"
grep -c "JPXDecode" /tmp/codec_usage.txt || echo "0"
echo ""
echo "Fax (CCITTFaxDecode - G3/G4):"
grep -c "CCITTFaxDecode" /tmp/codec_usage.txt || echo "0"
echo ""
echo "Flate (FlateDecode - zlib, always needed):"
grep -c "FlateDecode" /tmp/codec_usage.txt || echo "0"
echo ""
echo "=== Analysis Complete ==="
