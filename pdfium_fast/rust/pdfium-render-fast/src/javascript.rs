//! PDF JavaScript actions support
//!
//! Provides access to document-level JavaScript actions embedded in PDF files.
//!
//! JavaScript actions can be used to execute code when the document is opened,
//! printed, or on other events. This is useful for:
//!
//! - **Security scanning**: Detect potentially malicious JavaScript
//! - **Document analysis**: Understand document behavior
//! - **Form automation**: Inspect form calculation/validation scripts
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::Pdfium;
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//!
//! // Check for JavaScript (security concern)
//! if doc.has_javascript() {
//!     println!("WARNING: Document contains {} JavaScript actions",
//!         doc.javascript_count());
//!
//!     for js in doc.javascript_actions() {
//!         println!("Script '{}': {} bytes",
//!             js.name().unwrap_or_default(),
//!             js.script().unwrap_or_default().len());
//!     }
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use pdfium_sys::*;

/// A JavaScript action embedded in a PDF document.
///
/// JavaScript actions contain a name (usually the function name or event trigger)
/// and the script source code.
pub struct PdfJavaScriptAction {
    handle: FPDF_JAVASCRIPT_ACTION,
}

impl PdfJavaScriptAction {
    /// Create a new JavaScript action from a handle.
    pub(crate) fn new(handle: FPDF_JAVASCRIPT_ACTION) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Get the raw handle.
    pub fn handle(&self) -> FPDF_JAVASCRIPT_ACTION {
        self.handle
    }

    /// Get the name of the JavaScript action.
    ///
    /// This is typically the function name or the event that triggers the action
    /// (e.g., "Document/WillClose", "Document/WillPrint").
    pub fn name(&self) -> Option<String> {
        unsafe {
            // First call to get buffer size (in bytes, UTF-16)
            let size = FPDFJavaScriptAction_GetName(self.handle, std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer (size is in bytes, UTF-16 = 2 bytes per char)
            let mut buffer: Vec<u16> = vec![0; (size as usize) / 2];
            let actual_size = FPDFJavaScriptAction_GetName(self.handle, buffer.as_mut_ptr(), size);

            if actual_size == 0 {
                return None;
            }

            // Remove trailing nulls
            while buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf16(&buffer).ok()
        }
    }

    /// Get the JavaScript source code.
    ///
    /// Returns the raw JavaScript code as a string.
    pub fn script(&self) -> Option<String> {
        unsafe {
            // First call to get buffer size (in bytes, UTF-16)
            let size = FPDFJavaScriptAction_GetScript(self.handle, std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer (size is in bytes, UTF-16 = 2 bytes per char)
            let mut buffer: Vec<u16> = vec![0; (size as usize) / 2];
            let actual_size =
                FPDFJavaScriptAction_GetScript(self.handle, buffer.as_mut_ptr(), size);

            if actual_size == 0 {
                return None;
            }

            // Remove trailing nulls
            while buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf16(&buffer).ok()
        }
    }

    /// Get the script length in characters (not bytes).
    ///
    /// Useful for checking script size without allocating the full string.
    pub fn script_length(&self) -> usize {
        unsafe {
            let size = FPDFJavaScriptAction_GetScript(self.handle, std::ptr::null_mut(), 0);
            if size > 2 {
                // Each UTF-16 char is 2 bytes, subtract null terminator
                (size as usize / 2).saturating_sub(1)
            } else {
                0
            }
        }
    }

    /// Check if the script contains potentially dangerous patterns.
    ///
    /// This is a simple heuristic check for common attack vectors.
    /// It does NOT guarantee security - use a proper JavaScript analyzer
    /// for security-critical applications.
    ///
    /// Checks for:
    /// - Network access (`XMLHttpRequest`, `fetch`, `SOAP`)
    /// - File system access (`exportDataObject`, `saveAs`)
    /// - External program execution (`launchURL`, `app.launchURL`)
    /// - Form submission (`submitForm`)
    pub fn has_suspicious_patterns(&self) -> bool {
        if let Some(script) = self.script() {
            let script_lower = script.to_lowercase();
            let patterns = [
                "xmlhttprequest",
                "fetch(",
                "soap.",
                "exportdataobject",
                "saveas",
                "launchurl",
                "app.launchurl",
                "submitform",
                "importdataobject",
                "net.http",
                "util.readfileintostream",
            ];
            patterns.iter().any(|p| script_lower.contains(p))
        } else {
            false
        }
    }
}

impl Drop for PdfJavaScriptAction {
    fn drop(&mut self) {
        unsafe {
            FPDFDoc_CloseJavaScriptAction(self.handle);
        }
    }
}

/// Collection of JavaScript actions in a document.
pub struct PdfJavaScriptActions {
    doc_handle: FPDF_DOCUMENT,
    count: usize,
}

impl PdfJavaScriptActions {
    /// Create from a document handle.
    pub(crate) fn new(doc_handle: FPDF_DOCUMENT) -> Self {
        let count = unsafe { FPDFDoc_GetJavaScriptActionCount(doc_handle) };
        Self {
            doc_handle,
            count: if count > 0 { count as usize } else { 0 },
        }
    }

    /// Get the number of JavaScript actions.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Check if there are no JavaScript actions.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get a JavaScript action by index.
    pub fn get(&self, index: usize) -> Option<PdfJavaScriptAction> {
        if index >= self.count {
            return None;
        }

        let handle = unsafe { FPDFDoc_GetJavaScriptAction(self.doc_handle, index as i32) };
        PdfJavaScriptAction::new(handle)
    }

    /// Iterate over all JavaScript actions.
    pub fn iter(&self) -> PdfJavaScriptActionsIter<'_> {
        PdfJavaScriptActionsIter {
            actions: self,
            index: 0,
        }
    }

    /// Check if any action contains suspicious patterns.
    ///
    /// Quick security scan - returns true if ANY action has suspicious code.
    pub fn has_any_suspicious(&self) -> bool {
        self.iter().any(|js| js.has_suspicious_patterns())
    }

    /// Get all action names.
    pub fn names(&self) -> Vec<Option<String>> {
        self.iter().map(|js| js.name()).collect()
    }

    /// Get total script size in characters.
    pub fn total_script_length(&self) -> usize {
        self.iter().map(|js| js.script_length()).sum()
    }
}

/// Iterator over JavaScript actions.
pub struct PdfJavaScriptActionsIter<'a> {
    actions: &'a PdfJavaScriptActions,
    index: usize,
}

impl<'a> Iterator for PdfJavaScriptActionsIter<'a> {
    type Item = PdfJavaScriptAction;

    fn next(&mut self) -> Option<Self::Item> {
        let action = self.actions.get(self.index)?;
        self.index += 1;
        Some(action)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.actions.count.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for PdfJavaScriptActionsIter<'a> {}

/// Consuming iterator over JavaScript actions.
pub struct PdfJavaScriptActionsIntoIter {
    doc_handle: FPDF_DOCUMENT,
    index: usize,
    count: usize,
}

impl Iterator for PdfJavaScriptActionsIntoIter {
    type Item = PdfJavaScriptAction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let handle = unsafe { FPDFDoc_GetJavaScriptAction(self.doc_handle, self.index as i32) };
        self.index += 1;
        PdfJavaScriptAction::new(handle)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PdfJavaScriptActionsIntoIter {}

impl IntoIterator for PdfJavaScriptActions {
    type Item = PdfJavaScriptAction;
    type IntoIter = PdfJavaScriptActionsIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        PdfJavaScriptActionsIntoIter {
            doc_handle: self.doc_handle,
            index: 0,
            count: self.count,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_suspicious_patterns() {
        // Test various suspicious patterns
        let patterns = [
            "XMLHttpRequest",
            "fetch(",
            "exportDataObject",
            "app.launchURL",
            "submitForm",
        ];

        for pattern in patterns {
            let script = format!("function test() {{ {}('test'); }}", pattern);
            let lower = script.to_lowercase();
            assert!(
                [
                    "xmlhttprequest",
                    "fetch(",
                    "exportdataobject",
                    "app.launchurl",
                    "submitform"
                ]
                .iter()
                .any(|p| lower.contains(p)),
                "Pattern {} should be detected as suspicious",
                pattern
            );
        }
    }

    #[test]
    fn test_safe_patterns() {
        // These should NOT trigger suspicious detection
        let safe_scripts = [
            "function calculate() { return 1 + 1; }",
            "var x = this.getField('name').value;",
            "AFNumber_Format(2, 0, 0, 0, '', true);",
        ];

        for script in safe_scripts {
            let lower = script.to_lowercase();
            let suspicious = [
                "xmlhttprequest",
                "fetch(",
                "soap.",
                "exportdataobject",
                "saveas",
                "launchurl",
                "submitform",
            ]
            .iter()
            .any(|p| lower.contains(p));
            assert!(!suspicious, "Safe script should not be flagged: {}", script);
        }
    }
}
