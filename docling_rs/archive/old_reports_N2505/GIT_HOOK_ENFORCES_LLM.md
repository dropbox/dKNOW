# 🔒 GIT HOOK INSTALLED - ENFORCES LLM QUALITY FIXES

**User Directive:** "Force it to fix the quality work! Add a git commit hook! Need 100%!"

**Installed:** `.git/hooks/pre-commit` (active now)

---

## WHAT THE HOOK DOES

**BLOCKS commits unless:**
- ✅ Commit mentions fixing HTML, DXF, PPTX, or AsciiDoc
- ✅ Commit mentions LLM test path fixes
- ✅ OR it's documentation/cleanup cycle

**Worker CANNOT commit other work until quality issues fixed!**

---

## ENFORCEMENT

**Worker tries to commit test expansion:**
```bash
git commit -m "Add more DOCX tests"

❌❌❌ COMMIT BLOCKED ❌❌❌

LLM QUALITY MANDATE: 4 formats have quality issues!

Failing formats:
  - HTML: 68% (need 85%)
  - DXF: 57% (need 75%)
  - PPTX: 73% (need 85%)
  - AsciiDoc: 73% (need 85%)

YOU MUST work on fixing these issues!
```

**Worker tries to commit quality fix:**
```bash
git commit -m "Fix HTML parser nested table extraction"

✅ Commit addresses LLM quality issues - ALLOWED
```

---

## WORKER CANNOT IGNORE THIS

**Previous problem:** Worker ignored written directives
**Solution:** Git hook BLOCKS commits

**Worker must:**
1. Fix LLM test file paths (easy)
2. Fix HTML quality (68% → 85%+)
3. Fix DXF quality (57% → 75%+)
4. Fix PPTX quality (73% → 85%+)
5. Fix AsciiDoc quality (73% → 85%+)

**THEN:** Hook allows other commits

---

**This is ENFORCEMENT. Worker cannot bypass without --no-verify.**
