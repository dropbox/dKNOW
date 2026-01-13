//! Text extraction from PDF pages

use crate::error::Result;
use crate::link::PdfPageLinks;
use crate::page::JapaneseCharAnalysis;
use crate::search::{PdfSearchOptions, PdfTextSearch};
use pdfium_sys::*;

/// Text content of a PDF page.
///
/// Provides access to the text content, character positions, and word boundaries.
pub struct PdfPageText {
    handle: FPDF_TEXTPAGE,
}

impl PdfPageText {
    pub(crate) fn new(handle: FPDF_TEXTPAGE) -> Self {
        Self { handle }
    }

    /// Get the raw text page handle.
    pub fn handle(&self) -> FPDF_TEXTPAGE {
        self.handle
    }

    /// Get the number of characters on the page.
    pub fn char_count(&self) -> usize {
        unsafe { FPDFText_CountChars(self.handle) as usize }
    }

    /// Get all text from the page as a string.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// println!("{}", text.all());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn all(&self) -> String {
        let count = self.char_count();
        if count == 0 {
            return String::new();
        }

        // Get text as UTF-8
        // First get required buffer size
        let size = unsafe { FPDFText_GetText(self.handle, 0, count as i32, std::ptr::null_mut()) };

        if size <= 0 {
            return String::new();
        }

        // Allocate buffer (UTF-16LE, 2 bytes per char plus null terminator)
        let mut buffer: Vec<u16> = vec![0; size as usize];
        unsafe {
            FPDFText_GetText(self.handle, 0, count as i32, buffer.as_mut_ptr());
        }

        // Remove trailing null and convert to String
        while !buffer.is_empty() && buffer[buffer.len() - 1] == 0 {
            buffer.pop();
        }

        String::from_utf16_lossy(&buffer)
    }

    /// Get text from a specific range of character indices.
    ///
    /// # Arguments
    ///
    /// * `start` - Starting character index (inclusive)
    /// * `count` - Number of characters to extract
    pub fn range(&self, start: usize, count: usize) -> String {
        if count == 0 {
            return String::new();
        }

        let size = unsafe {
            FPDFText_GetText(
                self.handle,
                start as i32,
                count as i32,
                std::ptr::null_mut(),
            )
        };

        if size <= 0 {
            return String::new();
        }

        let mut buffer: Vec<u16> = vec![0; size as usize];
        unsafe {
            FPDFText_GetText(self.handle, start as i32, count as i32, buffer.as_mut_ptr());
        }

        while !buffer.is_empty() && buffer[buffer.len() - 1] == 0 {
            buffer.pop();
        }

        String::from_utf16_lossy(&buffer)
    }

    /// Get an iterator over all characters in the page.
    pub fn chars(&self) -> PdfChars<'_> {
        PdfChars {
            text: self,
            index: 0,
            count: self.char_count(),
        }
    }

    /// Get all characters with their text rise values.
    ///
    /// Returns a vector of tuples containing (character index, unicode char, rise value).
    /// This is useful for detecting superscripts and subscripts in scientific documents.
    ///
    /// - Positive rise indicates superscript (raised text)
    /// - Negative rise indicates subscript (lowered text)
    /// - Near-zero rise indicates normal baseline text
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// // Find superscripts
    /// for (idx, ch, rise) in text.chars_with_rise() {
    ///     if rise > 2.0 {
    ///         println!("Superscript at {}: {} (rise: {:.1}pt)", idx, ch, rise);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn chars_with_rise(&self) -> Vec<(usize, char, f32)> {
        self.chars()
            .map(|c| (c.index, c.unicode, c.text_rise()))
            .collect()
    }

    /// Get characters that appear to be superscripts.
    ///
    /// Returns characters with text rise above the given threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Minimum rise in points to consider as superscript (default: 2.0)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// for ch in text.superscripts(2.0) {
    ///     println!("Superscript: {} at index {}", ch.unicode, ch.index);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn superscripts(&self, threshold: f32) -> Vec<PdfChar> {
        self.chars()
            .filter(|c| c.is_superscript(threshold))
            .collect()
    }

    /// Get characters that appear to be subscripts.
    ///
    /// Returns characters with text rise below the negative threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Maximum rise (as positive value) to consider as subscript (default: 2.0)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// for ch in text.subscripts(2.0) {
    ///     println!("Subscript: {} at index {}", ch.unicode, ch.index);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn subscripts(&self, threshold: f32) -> Vec<PdfChar> {
        self.chars().filter(|c| c.is_subscript(threshold)).collect()
    }

    /// Get character information at a specific index.
    pub fn char_at(&self, index: usize) -> Option<PdfChar> {
        if index >= self.char_count() {
            return None;
        }

        let unicode = unsafe { FPDFText_GetUnicode(self.handle, index as i32) };
        if unicode == 0 {
            return None;
        }

        let mut left = 0.0f64;
        let mut bottom = 0.0f64;
        let mut right = 0.0f64;
        let mut top = 0.0f64;

        let ok = unsafe {
            FPDFText_GetCharBox(
                self.handle,
                index as i32,
                &mut left,
                &mut right,
                &mut bottom,
                &mut top,
            )
        };

        if ok == 0 {
            return None;
        }

        let font_size = unsafe { FPDFText_GetFontSize(self.handle, index as i32) };
        let angle = unsafe { FPDFText_GetCharAngle(self.handle, index as i32) };

        // Get character origin (baseline position)
        let mut origin_x = 0.0f64;
        let mut origin_y = 0.0f64;
        let origin_ok = unsafe {
            FPDFText_GetCharOrigin(self.handle, index as i32, &mut origin_x, &mut origin_y)
        };

        // If GetCharOrigin fails, use default values (left, bottom)
        if origin_ok == 0 {
            origin_x = left;
            origin_y = bottom;
        }

        Some(PdfChar {
            index,
            unicode: char::from_u32(unicode).unwrap_or('\u{FFFD}'),
            left,
            bottom,
            right,
            top,
            font_size,
            angle,
            origin_x,
            origin_y,
        })
    }

    /// Get the number of words on the page.
    ///
    /// Words are separated by whitespace and significant gaps.
    pub fn word_count(&self) -> usize {
        unsafe { FPDFText_CountWords(self.handle) as usize }
    }

    /// Extract all words from the page.
    ///
    /// Returns a vector of `PdfWord` containing word boundaries and positions.
    pub fn words(&self) -> Vec<PdfWord> {
        let count = self.word_count();
        if count == 0 {
            return Vec::new();
        }

        let mut words = Vec::with_capacity(count);

        // Allocate FPDF_WORD_INFO array
        let mut infos: Vec<FPDF_WORD_INFO> = vec![
            FPDF_WORD_INFO {
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
                top: 0.0,
                start_char: 0,
                end_char: 0,
                text_offset: 0,
                text_length: 0,
            };
            count
        ];

        // Also need a text buffer for the function
        let total_chars = self.char_count();
        let mut text_buffer: Vec<u16> = vec![0; total_chars + 1];

        let extracted = unsafe {
            FPDFText_ExtractWords(
                self.handle,
                infos.as_mut_ptr(),
                count as i32,
                text_buffer.as_mut_ptr(),
                text_buffer.len() as i32,
            )
        };

        for info in infos.iter().take(extracted as usize) {
            let start = info.start_char as usize;
            let end = info.end_char as usize;

            // Get text for this word from text_buffer using text_offset and text_length
            let text_start = info.text_offset as usize;
            let text_end = text_start + info.text_length as usize;
            let text = if text_end <= text_buffer.len() {
                let text_slice = &text_buffer[text_start..text_end];
                // Convert UTF-16 to String, filtering out null terminators
                text_slice
                    .iter()
                    .filter(|&&c| c != 0)
                    .map(|&c| char::from_u32(c as u32).unwrap_or('\u{FFFD}'))
                    .collect()
            } else {
                String::new()
            };

            words.push(PdfWord {
                text,
                left: info.left as f64,
                bottom: info.bottom as f64,
                right: info.right as f64,
                top: info.top as f64,
                start_char_index: start,
                end_char_index: end,
            });
        }

        words
    }

    /// Extract all text cells from the page in a single efficient call.
    ///
    /// Text cells are contiguous runs of text with consistent font properties.
    /// This method uses batch extraction (2-3 FFI calls vs 100-400 for char-by-char).
    ///
    /// # Returns
    ///
    /// A vector of `PdfTextCell` containing text, bounding boxes, and font metadata.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// for cell in text.cells() {
    ///     println!("'{}' at ({:.1}, {:.1}) size={:.1}pt bold={}",
    ///         cell.text, cell.left, cell.bottom,
    ///         cell.font_size, cell.is_bold());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn cells(&self) -> Vec<PdfTextCell> {
        // Get required buffer sizes
        let mut cell_count: i32 = 0;
        let mut text_chars: i32 = 0;
        let result = unsafe {
            FPDFText_GetAllCellsBufferSizes(self.handle, &mut cell_count, &mut text_chars)
        };

        if result == 0 || cell_count == 0 {
            return Vec::new();
        }

        // Allocate buffers
        let mut infos: Vec<FPDF_TEXT_CELL_INFO> = vec![
            FPDF_TEXT_CELL_INFO {
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
                top: 0.0,
                text_offset: 0,
                text_length: 0,
                font_size: 0.0,
                font_flags: 0,
                char_start: 0,
                char_count: 0,
            };
            cell_count as usize
        ];
        let mut text_buffer: Vec<u16> = vec![0; (text_chars + 1) as usize];

        // Extract all cells in a single call
        let extracted = unsafe {
            FPDFText_ExtractAllCells(
                self.handle,
                infos.as_mut_ptr(),
                cell_count,
                text_buffer.as_mut_ptr(),
                text_chars + 1,
            )
        };

        if extracted < 0 {
            return Vec::new();
        }

        // Convert to PdfTextCell
        let mut cells = Vec::with_capacity(extracted as usize);
        for info in infos.iter().take(extracted as usize) {
            // Get text from text_buffer using text_offset and text_length
            let text_start = info.text_offset as usize;
            let text_end = text_start + info.text_length as usize;
            let text = if text_end <= text_buffer.len() {
                let text_slice = &text_buffer[text_start..text_end];
                text_slice
                    .iter()
                    .filter(|&&c| c != 0)
                    .map(|&c| char::from_u32(c as u32).unwrap_or('\u{FFFD}'))
                    .collect()
            } else {
                String::new()
            };

            cells.push(PdfTextCell {
                text,
                left: info.left as f64,
                bottom: info.bottom as f64,
                right: info.right as f64,
                top: info.top as f64,
                font_size: info.font_size,
                font_flags: info.font_flags,
                char_start: info.char_start as usize,
                char_count: info.char_count as usize,
            });
        }

        cells
    }

    /// Get text within a rectangular region.
    ///
    /// # Arguments
    ///
    /// * `left`, `top`, `right`, `bottom` - Rectangle bounds in page coordinates
    pub fn text_in_rect(&self, left: f64, top: f64, right: f64, bottom: f64) -> String {
        // Get text from rectangle
        let size = unsafe {
            FPDFText_GetBoundedText(
                self.handle,
                left,
                top,
                right,
                bottom,
                std::ptr::null_mut(),
                0,
            )
        };

        if size <= 0 {
            return String::new();
        }

        let mut buffer: Vec<u16> = vec![0; size as usize];
        unsafe {
            FPDFText_GetBoundedText(
                self.handle,
                left,
                top,
                right,
                bottom,
                buffer.as_mut_ptr(),
                size,
            );
        }

        while !buffer.is_empty() && buffer[buffer.len() - 1] == 0 {
            buffer.pop();
        }

        String::from_utf16_lossy(&buffer)
    }

    /// Extract web links from the page.
    ///
    /// Returns a container for accessing URLs and their positions on the page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// let links = text.links()?;
    /// for link in links.all() {
    ///     println!("URL: {}", link.url);
    ///     for rect in &link.rects {
    ///         println!("  at ({:.1}, {:.1}) - ({:.1}, {:.1})",
    ///             rect.left, rect.bottom, rect.right, rect.top);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn links(&self) -> Result<PdfPageLinks> {
        PdfPageLinks::from_text_page(self.handle)
    }

    /// Convert a text index to an internal character index.
    ///
    /// PDFium has two different indexing systems:
    /// - **Text index**: Position in the extracted text string (from `FPDFText_GetText`)
    /// - **Char index**: Internal character list index (used by character-level APIs)
    ///
    /// This method converts from text index to char index.
    ///
    /// # Arguments
    ///
    /// * `text_index` - Index in the extracted text string
    ///
    /// # Returns
    ///
    /// The corresponding internal character index, or `None` if the index is invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// // Get the char index for text position 10
    /// if let Some(char_idx) = text.text_index_to_char_index(10) {
    ///     // Now we can use char_idx with character-level APIs
    ///     if let Some(ch) = text.char_at(char_idx) {
    ///         println!("Character at text index 10: {}", ch.unicode);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn text_index_to_char_index(&self, text_index: usize) -> Option<usize> {
        let result = unsafe {
            pdfium_sys::FPDFText_GetCharIndexFromTextIndex(self.handle, text_index as i32)
        };
        if result < 0 {
            None
        } else {
            Some(result as usize)
        }
    }

    /// Convert an internal character index to a text index.
    ///
    /// This is the reverse of [`Self::text_index_to_char_index`].
    ///
    /// # Arguments
    ///
    /// * `char_index` - Internal character index
    ///
    /// # Returns
    ///
    /// The corresponding text index, or `None` if the index is invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// // Get the text index for character position 5
    /// if let Some(text_idx) = text.char_index_to_text_index(5) {
    ///     // Use text_idx to extract text with range()
    ///     println!("Text at char index 5 starts at text index {}", text_idx);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn char_index_to_text_index(&self, char_index: usize) -> Option<usize> {
        let result = unsafe {
            pdfium_sys::FPDFText_GetTextIndexFromCharIndex(self.handle, char_index as i32)
        };
        if result < 0 {
            None
        } else {
            Some(result as usize)
        }
    }

    /// Search for text within the page.
    ///
    /// Returns a search context that can be used to find all occurrences
    /// of the search pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The text pattern to search for
    /// * `options` - Search options (case sensitivity, whole word, etc.)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfSearchOptions};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// // Case-insensitive search (default)
    /// let mut search = text.search("hello", PdfSearchOptions::default())?;
    /// while search.find_next() {
    ///     let start = search.match_start();
    ///     let count = search.match_count();
    ///     let matched_text = text.range(start as usize, count as usize);
    ///     println!("Found '{}' at index {}", matched_text, start);
    /// }
    ///
    /// // Case-sensitive whole word search
    /// let options = PdfSearchOptions::new().case_sensitive().whole_word();
    /// let mut search = text.search("World", options)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn search(&self, pattern: &str, options: PdfSearchOptions) -> Result<PdfTextSearch> {
        PdfTextSearch::new(self.handle, pattern, options, 0)
    }

    /// Search for text starting from a specific character index.
    ///
    /// Use `start_index = -1` to start from the end of the page (for backward search).
    pub fn search_from(
        &self,
        pattern: &str,
        options: PdfSearchOptions,
        start_index: i32,
    ) -> Result<PdfTextSearch> {
        PdfTextSearch::new(self.handle, pattern, options, start_index)
    }

    /// Analyze Japanese character types in this text.
    ///
    /// Counts hiragana, katakana, kanji, and other Japanese character categories.
    ///
    /// # Returns
    ///
    /// A `JapaneseCharAnalysis` containing character counts by category.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// let analysis = text.japanese_char_analysis();
    /// println!("Hiragana: {}, Katakana: {}, Kanji: {}",
    ///     analysis.hiragana_count, analysis.katakana_count, analysis.kanji_count);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn japanese_char_analysis(&self) -> JapaneseCharAnalysis {
        let mut analysis = JapaneseCharAnalysis::new();
        let text = self.all();
        for ch in text.chars() {
            analysis.analyze_char(ch);
        }
        analysis
    }
}

impl Drop for PdfPageText {
    fn drop(&mut self) {
        unsafe {
            FPDFText_ClosePage(self.handle);
        }
    }
}

/// Information about a single character.
#[derive(Debug, Clone)]
pub struct PdfChar {
    /// Character index in the page
    pub index: usize,
    /// Unicode character
    pub unicode: char,
    /// Left coordinate of bounding box
    pub left: f64,
    /// Bottom coordinate of bounding box
    pub bottom: f64,
    /// Right coordinate of bounding box
    pub right: f64,
    /// Top coordinate of bounding box
    pub top: f64,
    /// Font size in points
    pub font_size: f64,
    /// Character angle in radians
    pub angle: f32,
    /// Origin x-coordinate (baseline position)
    pub origin_x: f64,
    /// Origin y-coordinate (baseline position)
    pub origin_y: f64,
}

impl PdfChar {
    /// Get the width of the character bounding box.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// Get the height of the character bounding box.
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }

    /// Get the text rise value for this character.
    ///
    /// Text rise represents the vertical offset from the baseline.
    /// - Positive values indicate superscript (raised text)
    /// - Negative values indicate subscript (lowered text)
    /// - Zero or near-zero indicates normal text
    ///
    /// The rise is computed as the difference between the character's origin
    /// y-coordinate (baseline) and its bounding box bottom coordinate.
    ///
    /// # Returns
    ///
    /// The text rise in points. Positive = superscript, negative = subscript.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// for ch in text.chars() {
    ///     let rise = ch.text_rise();
    ///     if rise > 2.0 {
    ///         println!("Superscript: {} (rise: {:.1}pt)", ch.unicode, rise);
    ///     } else if rise < -2.0 {
    ///         println!("Subscript: {} (rise: {:.1}pt)", ch.unicode, rise);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn text_rise(&self) -> f32 {
        // Text rise is the difference between origin y and the expected baseline
        // For standard text, origin_y should be close to bottom
        // Superscripts have origin_y > bottom (raised baseline)
        // Subscripts have origin_y < bottom (lowered baseline)
        (self.origin_y - self.bottom) as f32
    }

    /// Check if this character appears to be a superscript.
    ///
    /// Returns true if the text rise exceeds the given threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Minimum rise in points to consider as superscript (default: 2.0)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// for ch in text.chars() {
    ///     if ch.is_superscript(2.0) {
    ///         println!("Superscript: {}", ch.unicode);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_superscript(&self, threshold: f32) -> bool {
        self.text_rise() > threshold
    }

    /// Check if this character appears to be a subscript.
    ///
    /// Returns true if the text rise is below the negative threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Maximum rise (as positive value) to consider as subscript (default: 2.0)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// for ch in text.chars() {
    ///     if ch.is_subscript(2.0) {
    ///         println!("Subscript: {}", ch.unicode);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_subscript(&self, threshold: f32) -> bool {
        self.text_rise() < -threshold
    }

    /// Check if this character is at the normal baseline (neither super nor subscript).
    ///
    /// Returns true if the absolute text rise is below the given threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Maximum absolute rise to consider as normal (default: 2.0)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    /// let text = page.text()?;
    ///
    /// for ch in text.chars() {
    ///     if ch.is_baseline(2.0) {
    ///         println!("Normal text: {}", ch.unicode);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_baseline(&self, threshold: f32) -> bool {
        self.text_rise().abs() <= threshold
    }
}

/// Iterator over characters in a page.
pub struct PdfChars<'a> {
    text: &'a PdfPageText,
    index: usize,
    count: usize,
}

impl<'a> Iterator for PdfChars<'a> {
    type Item = PdfChar;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let ch = self.text.char_at(self.index)?;
        self.index += 1;
        Some(ch)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.index;
        (remaining, Some(remaining))
    }
}

/// Information about a word extracted from the page.
#[derive(Debug, Clone)]
pub struct PdfWord {
    /// The word text
    pub text: String,
    /// Left coordinate of bounding box
    pub left: f64,
    /// Bottom coordinate of bounding box
    pub bottom: f64,
    /// Right coordinate of bounding box
    pub right: f64,
    /// Top coordinate of bounding box
    pub top: f64,
    /// Starting character index
    pub start_char_index: usize,
    /// Ending character index (exclusive)
    pub end_char_index: usize,
}

impl PdfWord {
    /// Get the width of the word bounding box.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// Get the height of the word bounding box.
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }
}

/// Font flag constants for text cells.
pub mod font_flags {
    /// Fixed-pitch (monospace) font
    pub const FIXED_PITCH: i32 = 0x0001;
    /// Serif font
    pub const SERIF: i32 = 0x0002;
    /// Symbolic font (special characters)
    pub const SYMBOLIC: i32 = 0x0004;
    /// Script (handwriting-style) font
    pub const SCRIPT: i32 = 0x0008;
    /// Non-symbolic font (standard characters)
    pub const NONSYMBOLIC: i32 = 0x0020;
    /// Italic font
    pub const ITALIC: i32 = 0x0040;
    /// All capitals font
    pub const ALLCAP: i32 = 0x10000;
    /// Small capitals font
    pub const SMALLCAP: i32 = 0x20000;
    /// Bold font
    pub const BOLD: i32 = 0x40000;
}

/// A text cell extracted from a PDF page.
///
/// Text cells are contiguous runs of text with consistent font properties.
/// This is more granular than words and includes font metadata.
#[derive(Debug, Clone)]
pub struct PdfTextCell {
    /// The text content
    pub text: String,
    /// Left coordinate of bounding box
    pub left: f64,
    /// Bottom coordinate of bounding box
    pub bottom: f64,
    /// Right coordinate of bounding box
    pub right: f64,
    /// Top coordinate of bounding box
    pub top: f64,
    /// Font size in points
    pub font_size: f32,
    /// Font flags (see font_flags module)
    pub font_flags: i32,
    /// Starting character index in the text page
    pub char_start: usize,
    /// Number of characters in this cell
    pub char_count: usize,
}

impl PdfTextCell {
    /// Get the width of the cell bounding box.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// Get the height of the cell bounding box.
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }

    /// Check if the font is bold.
    pub fn is_bold(&self) -> bool {
        self.font_flags & font_flags::BOLD != 0
    }

    /// Check if the font is italic.
    pub fn is_italic(&self) -> bool {
        self.font_flags & font_flags::ITALIC != 0
    }

    /// Check if the font is fixed-pitch (monospace).
    pub fn is_monospace(&self) -> bool {
        self.font_flags & font_flags::FIXED_PITCH != 0
    }

    /// Check if the font is serif.
    pub fn is_serif(&self) -> bool {
        self.font_flags & font_flags::SERIF != 0
    }
}
