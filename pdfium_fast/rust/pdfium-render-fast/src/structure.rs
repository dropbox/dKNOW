//! PDF document structure tree for accessibility.
//!
//! The structure tree represents the logical structure of a tagged PDF document,
//! providing accessibility information like headings, paragraphs, tables, etc.
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::Pdfium;
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("tagged.pdf", None)?;
//! let page = doc.page(0)?;
//!
//! if let Some(tree) = page.structure_tree() {
//!     println!("Structure tree has {} root elements", tree.child_count());
//!     for elem in tree.children() {
//!         println!("Element type: {}", elem.element_type().unwrap_or_default());
//!     }
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use pdfium_sys::*;
use std::ptr;

/// The structure tree of a PDF page.
///
/// Contains the root-level structure elements for the page.
/// This is obtained from `PdfPage::structure_tree()`.
pub struct PdfStructTree {
    handle: FPDF_STRUCTTREE,
}

impl PdfStructTree {
    /// Create a new structure tree from a PDFium handle.
    pub(crate) fn from_handle(handle: FPDF_STRUCTTREE) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Get the number of root-level children.
    pub fn child_count(&self) -> i32 {
        unsafe { FPDF_StructTree_CountChildren(self.handle) }
    }

    /// Get a child element at the specified index.
    ///
    /// Returns None if the index is out of bounds or the element is invalid.
    pub fn child(&self, index: usize) -> Option<PdfStructElement> {
        if index >= self.child_count() as usize {
            return None;
        }
        let elem = unsafe { FPDF_StructTree_GetChildAtIndex(self.handle, index as i32) };
        PdfStructElement::from_handle(elem)
    }

    /// Iterate over all root-level children.
    pub fn children(&self) -> PdfStructTreeChildIter<'_> {
        PdfStructTreeChildIter {
            tree: self,
            index: 0,
            count: self.child_count() as usize,
        }
    }

    /// Check if this structure tree is empty.
    pub fn is_empty(&self) -> bool {
        self.child_count() == 0
    }

    /// Recursively collect all elements in the tree (depth-first).
    pub fn all_elements(&self) -> Vec<PdfStructElement> {
        let mut result = Vec::new();
        for child in self.children() {
            Self::collect_recursive(&child, &mut result);
        }
        result
    }

    fn collect_recursive(elem: &PdfStructElement, result: &mut Vec<PdfStructElement>) {
        // Clone the element (creating a new reference)
        if let Some(cloned) = PdfStructElement::from_handle(elem.handle) {
            result.push(cloned);
        }
        for child in elem.children() {
            Self::collect_recursive(&child, result);
        }
    }
}

impl Drop for PdfStructTree {
    fn drop(&mut self) {
        unsafe {
            FPDF_StructTree_Close(self.handle);
        }
    }
}

/// Iterator over structure tree root children.
pub struct PdfStructTreeChildIter<'a> {
    tree: &'a PdfStructTree,
    index: usize,
    count: usize,
}

impl Iterator for PdfStructTreeChildIter<'_> {
    type Item = PdfStructElement;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let elem = self.tree.child(self.index);
        self.index += 1;
        elem
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PdfStructTreeChildIter<'_> {}

/// A structure element in the document structure tree.
///
/// Structure elements represent logical content like headings, paragraphs,
/// figures, tables, etc. They provide accessibility information.
pub struct PdfStructElement {
    handle: FPDF_STRUCTELEMENT,
}

impl PdfStructElement {
    /// Create a structure element from a PDFium handle.
    pub(crate) fn from_handle(handle: FPDF_STRUCTELEMENT) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Get the raw PDFium handle.
    pub fn handle(&self) -> FPDF_STRUCTELEMENT {
        self.handle
    }

    /// Get the structure element type (e.g., "P", "H1", "Table", "Figure").
    ///
    /// Returns None if the type cannot be retrieved.
    ///
    /// Common element types:
    /// - "Document": Root document element
    /// - "Part", "Art", "Sect", "Div": Grouping elements
    /// - "H", "H1"-"H6": Headings
    /// - "P": Paragraph
    /// - "L", "LI", "LBody": Lists
    /// - "Table", "TR", "TH", "TD": Tables
    /// - "Figure", "Formula": Illustrations
    /// - "Span", "Quote", "Code": Inline elements
    pub fn element_type(&self) -> Option<String> {
        self.get_string_property(|elem, buf, len| unsafe {
            FPDF_StructElement_GetType(elem, buf, len)
        })
    }

    /// Get the object type of this element.
    ///
    /// This is the PDF object type, not the structure type.
    pub fn object_type(&self) -> Option<String> {
        self.get_string_property(|elem, buf, len| unsafe {
            FPDF_StructElement_GetObjType(elem, buf, len)
        })
    }

    /// Get the title of this element.
    ///
    /// Titles provide human-readable names for structure elements.
    pub fn title(&self) -> Option<String> {
        self.get_string_property(|elem, buf, len| unsafe {
            FPDF_StructElement_GetTitle(elem, buf, len)
        })
    }

    /// Get the alternative text for this element.
    ///
    /// Alt text is used for accessibility, providing descriptions for
    /// images and other non-text content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// if let Some(tree) = page.structure_tree() {
    ///     for elem in tree.all_elements() {
    ///         if let Some(alt) = elem.alt_text() {
    ///             println!("Alt text: {}", alt);
    ///         }
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn alt_text(&self) -> Option<String> {
        self.get_string_property(|elem, buf, len| unsafe {
            FPDF_StructElement_GetAltText(elem, buf, len)
        })
    }

    /// Get the actual text content of this element.
    ///
    /// The actual text is the text content directly associated with this element.
    pub fn actual_text(&self) -> Option<String> {
        self.get_string_property(|elem, buf, len| unsafe {
            FPDF_StructElement_GetActualText(elem, buf, len)
        })
    }

    /// Get the ID of this element.
    ///
    /// IDs provide unique identifiers within the document.
    pub fn id(&self) -> Option<String> {
        self.get_string_property(|elem, buf, len| unsafe {
            FPDF_StructElement_GetID(elem, buf, len)
        })
    }

    /// Get the language of this element.
    ///
    /// Returns the BCP 47 language tag (e.g., "en-US", "ja-JP").
    pub fn language(&self) -> Option<String> {
        self.get_string_property(|elem, buf, len| unsafe {
            FPDF_StructElement_GetLang(elem, buf, len)
        })
    }

    /// Get a string attribute by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The attribute name to look up
    pub fn string_attribute(&self, name: &str) -> Option<String> {
        let c_name = std::ffi::CString::new(name).ok()?;

        // Get required buffer size
        let size = unsafe {
            FPDF_StructElement_GetStringAttribute(self.handle, c_name.as_ptr(), ptr::null_mut(), 0)
        };
        if size == 0 {
            return None;
        }

        // Get the string
        let mut buffer = vec![0u8; size as usize];
        let actual = unsafe {
            FPDF_StructElement_GetStringAttribute(
                self.handle,
                c_name.as_ptr(),
                buffer.as_mut_ptr() as *mut _,
                size,
            )
        };
        if actual == 0 {
            return None;
        }

        // Convert from UTF-16LE (PDFium uses UTF-16)
        Self::utf16le_to_string(&buffer)
    }

    /// Get the marked content ID for this element.
    ///
    /// Returns -1 if no marked content ID is present.
    pub fn marked_content_id(&self) -> i32 {
        unsafe { FPDF_StructElement_GetMarkedContentID(self.handle) }
    }

    /// Get the number of marked content IDs.
    pub fn marked_content_id_count(&self) -> i32 {
        unsafe { FPDF_StructElement_GetMarkedContentIdCount(self.handle) }
    }

    /// Get a marked content ID at a specific index.
    pub fn marked_content_id_at(&self, index: usize) -> Option<i32> {
        if index >= self.marked_content_id_count() as usize {
            return None;
        }
        let id = unsafe { FPDF_StructElement_GetMarkedContentIdAtIndex(self.handle, index as i32) };
        if id < 0 {
            None
        } else {
            Some(id)
        }
    }

    /// Get the number of children.
    pub fn child_count(&self) -> i32 {
        unsafe { FPDF_StructElement_CountChildren(self.handle) }
    }

    /// Get a child element at the specified index.
    pub fn child(&self, index: usize) -> Option<PdfStructElement> {
        if index >= self.child_count() as usize {
            return None;
        }
        let elem = unsafe { FPDF_StructElement_GetChildAtIndex(self.handle, index as i32) };
        Self::from_handle(elem)
    }

    /// Iterate over all children.
    pub fn children(&self) -> PdfStructElementChildIter<'_> {
        PdfStructElementChildIter {
            element: self,
            index: 0,
            count: self.child_count() as usize,
        }
    }

    /// Check if this element has children.
    pub fn has_children(&self) -> bool {
        self.child_count() > 0
    }

    /// Get the parent element.
    ///
    /// Returns None if this is a root element.
    pub fn parent(&self) -> Option<PdfStructElement> {
        let elem = unsafe { FPDF_StructElement_GetParent(self.handle) };
        Self::from_handle(elem)
    }

    /// Get the number of attributes.
    pub fn attribute_count(&self) -> i32 {
        unsafe { FPDF_StructElement_GetAttributeCount(self.handle) }
    }

    /// Get an attribute object at the specified index.
    pub fn attribute(&self, index: usize) -> Option<PdfStructAttribute> {
        if index >= self.attribute_count() as usize {
            return None;
        }
        let attr = unsafe { FPDF_StructElement_GetAttributeAtIndex(self.handle, index as i32) };
        PdfStructAttribute::from_handle(attr)
    }

    /// Iterate over all attributes.
    pub fn attributes(&self) -> impl Iterator<Item = PdfStructAttribute> + '_ {
        (0..self.attribute_count() as usize).filter_map(|i| self.attribute(i))
    }

    /// Check if this is a heading element (H, H1-H6).
    pub fn is_heading(&self) -> bool {
        self.element_type()
            .map(|t| {
                t == "H"
                    || t.starts_with("H")
                        && t.len() == 2
                        && t.chars()
                            .nth(1)
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Check if this is a paragraph element (P).
    pub fn is_paragraph(&self) -> bool {
        self.element_type().map(|t| t == "P").unwrap_or(false)
    }

    /// Check if this is a table element (Table).
    pub fn is_table(&self) -> bool {
        self.element_type().map(|t| t == "Table").unwrap_or(false)
    }

    /// Check if this is a figure element (Figure).
    pub fn is_figure(&self) -> bool {
        self.element_type().map(|t| t == "Figure").unwrap_or(false)
    }

    /// Check if this is a list element (L).
    pub fn is_list(&self) -> bool {
        self.element_type().map(|t| t == "L").unwrap_or(false)
    }

    // Helper to get string properties from PDFium
    fn get_string_property<F>(&self, getter: F) -> Option<String>
    where
        F: Fn(FPDF_STRUCTELEMENT, *mut std::ffi::c_void, u64) -> u64,
    {
        // Get required buffer size
        let size = getter(self.handle, ptr::null_mut(), 0);
        if size == 0 {
            return None;
        }

        // Get the string
        let mut buffer = vec![0u8; size as usize];
        let actual = getter(self.handle, buffer.as_mut_ptr() as *mut _, size);
        if actual == 0 {
            return None;
        }

        // Convert from UTF-16LE
        Self::utf16le_to_string(&buffer)
    }

    // Convert UTF-16LE bytes to String
    fn utf16le_to_string(bytes: &[u8]) -> Option<String> {
        // PDFium returns UTF-16LE with null terminator
        if bytes.len() < 2 {
            return None;
        }

        // Convert byte pairs to u16
        let u16_vec: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .take_while(|&c| c != 0)
            .collect();

        String::from_utf16(&u16_vec).ok()
    }
}

/// Iterator over structure element children.
pub struct PdfStructElementChildIter<'a> {
    element: &'a PdfStructElement,
    index: usize,
    count: usize,
}

impl Iterator for PdfStructElementChildIter<'_> {
    type Item = PdfStructElement;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let elem = self.element.child(self.index);
        self.index += 1;
        elem
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PdfStructElementChildIter<'_> {}

/// A structure element attribute.
///
/// Attributes provide additional information about structure elements.
pub struct PdfStructAttribute {
    handle: FPDF_STRUCTELEMENT_ATTR,
}

impl PdfStructAttribute {
    /// Create an attribute from a PDFium handle.
    pub(crate) fn from_handle(handle: FPDF_STRUCTELEMENT_ATTR) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Get the number of key-value pairs in this attribute.
    pub fn count(&self) -> i32 {
        unsafe { FPDF_StructElement_Attr_GetCount(self.handle) }
    }

    /// Get an attribute name at the specified index.
    pub fn name_at(&self, index: usize) -> Option<String> {
        if index >= self.count() as usize {
            return None;
        }

        // Get required buffer size
        let mut out_len = 0u64;
        unsafe {
            FPDF_StructElement_Attr_GetName(
                self.handle,
                index as i32,
                ptr::null_mut(),
                0,
                &mut out_len,
            );
        }
        if out_len == 0 {
            return None;
        }

        // Get the name
        let mut buffer = vec![0u8; out_len as usize];
        let mut actual_len = 0u64;
        let result = unsafe {
            FPDF_StructElement_Attr_GetName(
                self.handle,
                index as i32,
                buffer.as_mut_ptr() as *mut _,
                out_len,
                &mut actual_len,
            )
        };
        if result == 0 {
            return None;
        }

        // Convert from null-terminated string
        buffer.truncate(actual_len as usize);
        if buffer.last() == Some(&0) {
            buffer.pop();
        }
        String::from_utf8(buffer).ok()
    }

    /// Get an attribute value by name.
    pub fn value(&self, name: &str) -> Option<PdfStructAttributeValue> {
        let c_name = std::ffi::CString::new(name).ok()?;
        let value = unsafe { FPDF_StructElement_Attr_GetValue(self.handle, c_name.as_ptr()) };
        PdfStructAttributeValue::from_handle(value)
    }

    /// Iterate over all name-value pairs.
    pub fn entries(
        &self,
    ) -> impl Iterator<Item = (Option<String>, Option<PdfStructAttributeValue>)> + '_ {
        (0..self.count() as usize).map(move |i| {
            let name = self.name_at(i);
            let value = name.as_ref().and_then(|n| self.value(n));
            (name, value)
        })
    }
}

/// Type of a structure attribute value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfStructAttributeValueType {
    /// Unknown type
    Unknown,
    /// Boolean value
    Boolean,
    /// Numeric value
    Number,
    /// String value
    String,
    /// Binary blob value
    Blob,
    /// Array of values
    Array,
}

/// A structure attribute value.
pub struct PdfStructAttributeValue {
    handle: FPDF_STRUCTELEMENT_ATTR_VALUE,
}

impl PdfStructAttributeValue {
    /// Create a value from a PDFium handle.
    pub(crate) fn from_handle(handle: FPDF_STRUCTELEMENT_ATTR_VALUE) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Get the type of this value.
    pub fn value_type(&self) -> PdfStructAttributeValueType {
        let t = unsafe { FPDF_StructElement_Attr_GetType(self.handle) };
        match t as u32 {
            0 => PdfStructAttributeValueType::Unknown,
            1 => PdfStructAttributeValueType::Boolean,
            2 => PdfStructAttributeValueType::Number,
            3 => PdfStructAttributeValueType::String,
            4 => PdfStructAttributeValueType::Blob,
            5 => PdfStructAttributeValueType::Array,
            _ => PdfStructAttributeValueType::Unknown,
        }
    }

    /// Get the boolean value (if this is a boolean).
    pub fn as_bool(&self) -> Option<bool> {
        let mut value = 0;
        let result = unsafe { FPDF_StructElement_Attr_GetBooleanValue(self.handle, &mut value) };
        if result != 0 {
            Some(value != 0)
        } else {
            None
        }
    }

    /// Get the numeric value (if this is a number).
    pub fn as_number(&self) -> Option<f32> {
        let mut value = 0.0f32;
        let result = unsafe { FPDF_StructElement_Attr_GetNumberValue(self.handle, &mut value) };
        if result != 0 {
            Some(value)
        } else {
            None
        }
    }

    /// Get the string value (if this is a string).
    pub fn as_string(&self) -> Option<String> {
        // Get required buffer size
        let mut out_len = 0u64;
        unsafe {
            FPDF_StructElement_Attr_GetStringValue(self.handle, ptr::null_mut(), 0, &mut out_len);
        }
        if out_len == 0 {
            return None;
        }

        // Get the string
        let mut buffer = vec![0u8; out_len as usize];
        let mut actual_len = 0u64;
        let result = unsafe {
            FPDF_StructElement_Attr_GetStringValue(
                self.handle,
                buffer.as_mut_ptr() as *mut _,
                out_len,
                &mut actual_len,
            )
        };
        if result == 0 {
            return None;
        }

        // Convert from UTF-16LE
        buffer.truncate(actual_len as usize);
        PdfStructElement::utf16le_to_string(&buffer)
    }

    /// Get the blob value (if this is a blob).
    pub fn as_blob(&self) -> Option<Vec<u8>> {
        // Get required buffer size
        let mut out_len = 0u64;
        unsafe {
            FPDF_StructElement_Attr_GetBlobValue(self.handle, ptr::null_mut(), 0, &mut out_len);
        }
        if out_len == 0 {
            return None;
        }

        // Get the blob
        let mut buffer = vec![0u8; out_len as usize];
        let mut actual_len = 0u64;
        let result = unsafe {
            FPDF_StructElement_Attr_GetBlobValue(
                self.handle,
                buffer.as_mut_ptr() as *mut _,
                out_len,
                &mut actual_len,
            )
        };
        if result == 0 {
            return None;
        }

        buffer.truncate(actual_len as usize);
        Some(buffer)
    }

    /// Get the number of array children (if this is an array).
    pub fn array_count(&self) -> i32 {
        unsafe { FPDF_StructElement_Attr_CountChildren(self.handle) }
    }

    /// Get an array child at the specified index.
    pub fn array_child(&self, index: usize) -> Option<PdfStructAttributeValue> {
        if index >= self.array_count() as usize {
            return None;
        }
        let value = unsafe { FPDF_StructElement_Attr_GetChildAtIndex(self.handle, index as i32) };
        Self::from_handle(value)
    }

    /// Iterate over array children (if this is an array).
    pub fn array_iter(&self) -> impl Iterator<Item = PdfStructAttributeValue> + '_ {
        (0..self.array_count() as usize).filter_map(|i| self.array_child(i))
    }
}

// ============================================================================
// Tagged Table Extraction
// ============================================================================

/// A table extracted from a tagged PDF structure tree.
///
/// This allows docling to extract table structure without ML inference
/// for tagged PDFs (~40% of enterprise documents).
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::{Pdfium, TaggedTable};
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("tagged.pdf", None)?;
///
/// if doc.is_tagged() {
///     let page = doc.page(0)?;
///     if let Some(tree) = page.structure_tree() {
///         // Find all tables in the structure tree
///         for elem in tree.children() {
///             if elem.is_table() {
///                 if let Some(table) = TaggedTable::from_element(&elem) {
///                     println!("Table: {} rows x {} cols", table.row_count(), table.col_count());
///                     for row in &table.rows {
///                         for cell in &row.cells {
///                             print!("[{}] ", cell.text);
///                         }
///                         println!();
///                     }
///                 }
///             }
///         }
///     }
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct TaggedTable {
    /// Rows of the table
    pub rows: Vec<TaggedTableRow>,
    /// Number of header rows
    pub header_row_count: usize,
}

/// A row in a tagged table.
#[derive(Debug, Clone)]
pub struct TaggedTableRow {
    /// Cells in this row
    pub cells: Vec<TaggedTableCell>,
    /// Whether this is a header row
    pub is_header: bool,
}

/// A cell in a tagged table.
#[derive(Debug, Clone)]
pub struct TaggedTableCell {
    /// Text content of the cell
    pub text: String,
    /// Whether this is a header cell
    pub is_header: bool,
    /// Number of rows this cell spans
    pub row_span: i32,
    /// Number of columns this cell spans
    pub col_span: i32,
}

impl TaggedTable {
    /// Extract a table from a tagged PDF structure element.
    ///
    /// Returns None if the element is not a valid table structure.
    pub fn from_element(table_elem: &PdfStructElement) -> Option<Self> {
        if !table_elem.is_table() {
            return None;
        }

        let mut rows = Vec::new();
        let mut header_row_count = 0;

        // Iterate over children to find rows
        for child in table_elem.children() {
            let elem_type = child.element_type().unwrap_or_default();

            match elem_type.as_str() {
                "TR" => {
                    let row = Self::extract_row(&child);
                    if row.is_header {
                        header_row_count += 1;
                    }
                    rows.push(row);
                }
                "THead" => {
                    // Table header section
                    for row_child in child.children() {
                        if row_child.element_type().as_deref() == Some("TR") {
                            let mut row = Self::extract_row(&row_child);
                            row.is_header = true;
                            header_row_count += 1;
                            rows.push(row);
                        }
                    }
                }
                "TBody" => {
                    // Table body section
                    for row_child in child.children() {
                        if row_child.element_type().as_deref() == Some("TR") {
                            rows.push(Self::extract_row(&row_child));
                        }
                    }
                }
                "TFoot" => {
                    // Table footer section
                    for row_child in child.children() {
                        if row_child.element_type().as_deref() == Some("TR") {
                            rows.push(Self::extract_row(&row_child));
                        }
                    }
                }
                _ => {
                    // Check if it's a row-like element
                    if Self::has_cell_children(&child) {
                        rows.push(Self::extract_row(&child));
                    }
                }
            }
        }

        if rows.is_empty() {
            return None;
        }

        Some(TaggedTable {
            rows,
            header_row_count,
        })
    }

    fn extract_row(row_elem: &PdfStructElement) -> TaggedTableRow {
        let mut cells = Vec::new();
        let mut is_header = false;

        for cell_child in row_elem.children() {
            let cell = Self::extract_cell(&cell_child);
            if cell.is_header {
                is_header = true;
            }
            cells.push(cell);
        }

        TaggedTableRow { cells, is_header }
    }

    fn extract_cell(cell_elem: &PdfStructElement) -> TaggedTableCell {
        let elem_type = cell_elem.element_type().unwrap_or_default();
        let is_header = elem_type == "TH";

        // Get text content by concatenating all actual_text and alt_text
        let mut text = String::new();
        Self::collect_text(cell_elem, &mut text);

        // Note: PDFium attribute API is index-based, not name-based
        // For simplicity, default to span of 1 for both row and column
        // Full span detection would require iterating through attributes
        let row_span = 1;
        let col_span = 1;

        TaggedTableCell {
            text: text.trim().to_string(),
            is_header,
            row_span,
            col_span,
        }
    }

    fn collect_text(elem: &PdfStructElement, result: &mut String) {
        // Try actual_text first
        if let Some(text) = elem.actual_text() {
            if !text.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&text);
                return;
            }
        }

        // Try alt_text
        if let Some(text) = elem.alt_text() {
            if !text.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&text);
                return;
            }
        }

        // Recurse into children
        for child in elem.children() {
            Self::collect_text(&child, result);
        }
    }

    fn has_cell_children(elem: &PdfStructElement) -> bool {
        for child in elem.children() {
            if let Some(t) = child.element_type() {
                if t == "TD" || t == "TH" {
                    return true;
                }
            }
        }
        false
    }

    /// Get the number of rows in the table.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get the number of columns (based on first row).
    pub fn col_count(&self) -> usize {
        self.rows.first().map(|r| r.cells.len()).unwrap_or(0)
    }

    /// Get header rows only.
    pub fn header_rows(&self) -> &[TaggedTableRow] {
        &self.rows[..self.header_row_count]
    }

    /// Get data rows only (non-header).
    pub fn data_rows(&self) -> &[TaggedTableRow] {
        &self.rows[self.header_row_count..]
    }
}
