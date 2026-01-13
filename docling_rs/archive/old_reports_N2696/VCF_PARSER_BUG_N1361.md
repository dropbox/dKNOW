# VCF Parser Bug Report - N=1361

## Problem

**Format**: VCF (vCard - contact files)
**LLM Score**: 0% (consistent across test runs)
**Test File**: `test-corpus/email/vcf/business_card.vcf`

## Symptoms

**LLM Findings**:
> "Missing fields such as address (ADR), URL, and potentially others. Metadata such as version and encoding are not captured."

**Test File Contains**:
- ✅ `ADR;TYPE=work:Suite 400;Building 5;789 Innovation Drive;Boston;MA;02115;USA`
- ✅ `URL:https://biotech.com/researchers/sarah-chen`
- ✅ `URL;TYPE=linkedin:https://linkedin.com/in/sarahchen`
- ✅ `VERSION:4.0`
- ✅ `BDAY:19850315`
- ✅ `GENDER:F`
- ✅ `LANG;PREF=1:en`
- ✅ `CATEGORIES:Research,Science,Biotechnology`

**Parser Output**: 5,138 chars of JSON (so it's producing something)

## Root Cause

**Likely Issue**: VCF parser not extracting all vCard fields

**Evidence**:
1. Parser produces 5KB JSON (not empty)
2. LLM says specific fields missing (ADR, URL)
3. Test file contains those fields
4. Score: 0% (all categories failed)

**Conclusion**: Parser implementation incomplete - only extracts subset of vCard fields

## Investigation Needed

**Check Parser Implementation**:
1. Which vCard fields are supported?
2. Is ADR field parsing implemented?
3. Is URL field parsing implemented?
4. Are structured fields (ADR has 7 components) being parsed correctly?

**Parser Location**: Likely in `crates/docling-backend/src/email/vcf.rs` or similar

## Expected Behavior

**A complete vCard parser should extract**:
- ✅ FN (Full Name)
- ✅ N (Name components: family, given, additional, prefix, suffix)
- ✅ EMAIL (with TYPE parameter)
- ✅ TEL (with TYPE parameter)
- ✅ ORG (Organization)
- ✅ TITLE (Job title)
- ❌ ADR (Address: PO box, extended, street, city, region, postal, country)
- ❌ URL (with optional TYPE)
- ✅ NOTE
- ✅ CATEGORIES
- ❌ BDAY (Birthday)
- ❌ LANG (Language preference)
- ❌ GENDER
- ✅ VERSION

**Based on LLM feedback, likely missing**:
- ADR (structured address)
- URL (web links)
- BDAY, LANG, GENDER (less critical but part of spec)

## Fix Priority

**Severity**: Medium
- Format works (produces output)
- Missing important fields (ADR, URL common in business cards)
- 0% score indicates substantial incompleteness

**Effort**: Low-Medium
- vCard is well-specified (RFC 6350)
- Likely just need to add field handlers
- Structured fields (ADR) slightly more complex

**Impact**: Medium
- vCard common for contact exchange
- Missing ADR makes output less useful
- But not a critical format

## Recommended Fix

1. **Read parser code**: Find VCF parser implementation
2. **Check field support**: List which fields are currently extracted
3. **Add missing fields**: Implement ADR, URL, BDAY, LANG, GENDER
4. **Test**: Verify business_card.vcf extracts all fields
5. **Re-run LLM test**: Should score >90%

**Estimated Time**: 1-2 hours

## Related Issues

**Other Zero-Score Formats** (possible similar issues):
- 7Z: 0% (archive not extracting files?)
- RAR: 0% (archive not extracting files?)
- FB2: 0% (ebook not extracting chapters?)
- TEX: 0% (LaTeX not parsing equations?)

**Pattern**: Zero scores often indicate incomplete parser implementations, not infrastructure issues

## Next Steps

For next AI (N=1362+):
1. Read this report
2. Investigate VCF parser code
3. Add missing field support
4. Re-test and verify improvement
5. Consider investigating other zero-score formats similarly
