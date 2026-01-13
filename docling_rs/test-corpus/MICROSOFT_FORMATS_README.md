# Microsoft Format Test Files

This directory contains test files for three Microsoft formats that are not commonly supported but use similar underlying technologies.

## File Inventory

### Microsoft Project (.mpp) - 5 files
Location: `test-corpus/microsoft-project/`
Total Size: 832 KB

1. **sample1_2019.mpp** (251 KB)
   - Format: Microsoft Project 2019 (mpp14)
   - Author: jon
   - Created: Oct 18, 2018
   - Content: Resource flags test project

2. **sample2_2010.mpp** (211 KB)
   - Format: Microsoft Project 2010 (mpp12)
   - Author: Project User
   - Created: Mar 8, 2017

3. **sample3_2007.mpp** (139 KB)
   - Format: Microsoft Project 2007 (mpp9)
   - Author: jon.iles@bcs.org.uk
   - Created: Mar 8, 2017

4. **sample4_2003.mpp** (131 KB)
   - Format: Microsoft Project 2003 (mpp9)
   - Author: Project User
   - Created: Mar 8, 2017

5. **sample5_2000.mpp** (93 KB)
   - Format: Microsoft Project 2000 (mpp9)
   - Author: Project User
   - Created: Mar 8, 2017

**Source:** MPXJ Project (https://github.com/joniles/mpxj)
**File Type:** OLE Compound Document Format (CFB)
**Verification:** All files verified with `file` command as valid Composite Document File V2

### Microsoft Access (.mdb/.accdb) - 5 files
Location: `test-corpus/microsoft-access/`
Total Size: 1.6 MB

1. **sample1.mdb** (64 KB)
   - Format: Jet 4.0 (Access 2000-2003)
   - Source: adox_jet4.mdb

2. **sample2.accdb** (512 KB)
   - Format: Access 2007+ (ACCDB)
   - Source: linkeeTest.accdb

3. **sample3.accdb** (436 KB)
   - Format: Access 2010 (ACCDB)
   - Source: testV2010.accdb

4. **sample4.accdb** (384 KB)
   - Format: Access 2007 (ACCDB)
   - Source: test2V2007.accdb

5. **sample5.mdb** (204 KB)
   - Format: Jet 4.0 (Access 2000)
   - Source: V2000 test database

**Source:** Jackcess Project (https://github.com/jahlborn/jackcess)
**File Type:** Microsoft Access Database
**Verification:** All files verified with `file` command as Microsoft Access Database

### Microsoft OneNote (.one) - 5 files
Location: `test-corpus/microsoft-onenote/`
Total Size: 20 KB

1. **sample1.one** (2.0 KB)
2. **sample2.one** (2.0 KB)
3. **sample3.one** (2.0 KB)
4. **sample4.one** (2.0 KB)
5. **sample5.one** (2.0 KB)

**Source:** Synthetically generated (valid OLE Compound Documents)
**File Type:** OLE Compound Document Format (CFB)
**Verification:** All files verified with `file` command as Composite Document File V2
**Note:** These are minimal valid OneNote files created programmatically due to the extreme rarity of real OneNote files in public repositories. They contain valid OLE headers and basic OneNote structure but minimal content.

## Technical Details

### Common File Format
All three formats use Microsoft's **OLE Compound Document Format** (also known as Compound File Binary Format - CFB):

- **Signature:** `D0 CF 11 E0 A1 B1 1A E1`
- **Structure:** Container format with multiple internal streams
- **Similar to:** ZIP-like archive with hierarchical storage

### Format Characteristics

**Microsoft Project (.mpp):**
- Project plans, Gantt charts, resource allocation
- Binary format with complex internal structure
- Multiple versions (2000, 2003, 2007, 2010, 2019+)
- Requires specialized parsers (e.g., MPXJ library)

**Microsoft Access (.mdb/.accdb):**
- Relational database files
- Contains tables, queries, forms, reports, macros
- Two main formats:
  - .mdb (Jet Database Engine, Access 97-2003)
  - .accdb (ACE Database Engine, Access 2007+)
- Well-documented format with open-source parsers

**Microsoft OneNote (.one):**
- Digital notebook files
- Hierarchical note structure
- Embedded media, handwriting, attachments
- Most complex of the three formats
- Limited public documentation and tooling

## Usage

These test files are provided for:

1. **Format detection testing** - Verify file type identification
2. **Parser development** - Develop support for these formats
3. **Conversion testing** - Test document conversion pipelines
4. **Compatibility testing** - Ensure handling of legacy file formats

## Limitations

### Microsoft Project Files
- Test files are from 2017-2018
- Generated from MPXJ test suite
- Focus on resource flag testing
- May not represent typical user-created projects

### Microsoft Access Files
- Test files are from Jackcess test suite
- May contain minimal or synthetic data
- Various Access versions represented (2000-2010)
- Real-world databases may be more complex

### Microsoft OneNote Files
- **Synthetically generated** - not real user documents
- Minimal content structure
- Valid file format but simplified
- Real OneNote files are significantly more complex
- Use caution when testing against these files

## Verification Commands

```bash
# Verify all file types
file test-corpus/microsoft-project/*.mpp
file test-corpus/microsoft-access/*.{mdb,accdb}
file test-corpus/microsoft-onenote/*.one

# Check file sizes
ls -lh test-corpus/microsoft-project/
ls -lh test-corpus/microsoft-access/
ls -lh test-corpus/microsoft-onenote/

# Calculate total sizes
du -sh test-corpus/microsoft-*/
```

## Future Improvements

1. **Microsoft Project:**
   - Obtain more diverse real-world project files
   - Add files with complex dependencies and constraints
   - Include files with custom fields and calendars

2. **Microsoft Access:**
   - Add databases with more complex schemas
   - Include files with relationships and indexes
   - Test encrypted database files

3. **Microsoft OneNote:**
   - **Critical:** Replace synthetic files with real OneNote documents
   - Obtain files from various OneNote versions (2010, 2013, 2016, 365)
   - Include notebooks with images, tables, and attachments
   - Add files with sections and page hierarchies

## References

- [MS-CFB: Compound File Binary Format](https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/)
- [MPXJ Project](https://github.com/joniles/mpxj) - Microsoft Project file parser
- [Jackcess](https://github.com/jahlborn/jackcess) - Microsoft Access database parser
- [MS-ONESTORE: OneNote File Format](https://docs.microsoft.com/en-us/openspecs/office_file_formats/ms-onestore/)

## License

Test files sourced from open-source projects (MPXJ, Jackcess) retain their original licenses.
Synthetically generated files are provided without restriction for testing purposes.
