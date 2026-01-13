//! PDF form field support
//!
//! This module provides access to interactive PDF forms (AcroForms).
//!
//! # Form Field Types
//!
//! - Text fields: User-editable text input
//! - Checkboxes and radio buttons: Boolean/choice selection
//! - Combo boxes and list boxes: Single or multiple selection from options
//! - Push buttons: Trigger actions
//! - Signature fields: Digital signatures
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::Pdfium;
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("form.pdf", None)?;
//! let page = doc.page(0)?;
//!
//! for field in page.form_fields() {
//!     println!("Field: {} = {}", field.name, field.value);
//!     if field.field_type.is_choice() {
//!         for (i, option) in field.options.iter().enumerate() {
//!             println!("  Option {}: {} (selected: {})", i, option.label, option.selected);
//!         }
//!     }
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use pdfium_sys::*;

/// Type of form field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PdfFormFieldType {
    /// Unknown field type
    Unknown = FPDF_FORMFIELD_UNKNOWN,
    /// Push button
    PushButton = FPDF_FORMFIELD_PUSHBUTTON,
    /// Checkbox
    CheckBox = FPDF_FORMFIELD_CHECKBOX,
    /// Radio button
    RadioButton = FPDF_FORMFIELD_RADIOBUTTON,
    /// Combo box (dropdown)
    ComboBox = FPDF_FORMFIELD_COMBOBOX,
    /// List box (multi-select)
    ListBox = FPDF_FORMFIELD_LISTBOX,
    /// Text field
    TextField = FPDF_FORMFIELD_TEXTFIELD,
    /// Digital signature field
    Signature = FPDF_FORMFIELD_SIGNATURE,
}

impl PdfFormFieldType {
    /// Create field type from raw PDFium value.
    pub fn from_raw(value: i32) -> Self {
        match value as u32 {
            FPDF_FORMFIELD_PUSHBUTTON => Self::PushButton,
            FPDF_FORMFIELD_CHECKBOX => Self::CheckBox,
            FPDF_FORMFIELD_RADIOBUTTON => Self::RadioButton,
            FPDF_FORMFIELD_COMBOBOX => Self::ComboBox,
            FPDF_FORMFIELD_LISTBOX => Self::ListBox,
            FPDF_FORMFIELD_TEXTFIELD => Self::TextField,
            FPDF_FORMFIELD_SIGNATURE => Self::Signature,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a button-type field.
    pub fn is_button(&self) -> bool {
        matches!(self, Self::PushButton | Self::CheckBox | Self::RadioButton)
    }

    /// Check if this is a choice field (combo/list).
    pub fn is_choice(&self) -> bool {
        matches!(self, Self::ComboBox | Self::ListBox)
    }

    /// Check if this is a text input field.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::TextField)
    }
}

/// Type of PDF form (AcroForm vs XFA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfFormType {
    /// No form or unknown type
    None,
    /// Standard AcroForm
    AcroForm,
    /// XFA form (full XFA)
    XfaFull,
    /// XFA foreground form
    XfaForeground,
}

impl PdfFormType {
    /// Create form type from raw PDFium value.
    pub fn from_raw(value: i32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::AcroForm,
            2 => Self::XfaFull,
            3 => Self::XfaForeground,
            _ => Self::None,
        }
    }
}

/// Form field flags (from PDF Reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormFieldFlags(pub u32);

impl FormFieldFlags {
    /// No flags set.
    pub const NONE: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_NONE);
    /// Field is read-only.
    pub const READ_ONLY: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_READONLY);
    /// Field is required.
    pub const REQUIRED: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_REQUIRED);
    /// Field value should not be exported.
    pub const NO_EXPORT: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_NOEXPORT);
    /// Text field: Allow multi-line text.
    pub const TEXT_MULTILINE: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_TEXT_MULTILINE);
    /// Text field: Password field (obscured text).
    pub const TEXT_PASSWORD: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_TEXT_PASSWORD);
    /// Choice field: Combo box (not list box).
    pub const CHOICE_COMBO: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_CHOICE_COMBO);
    /// Choice field: Editable combo box.
    pub const CHOICE_EDIT: FormFieldFlags = FormFieldFlags(FPDF_FORMFLAG_CHOICE_EDIT);
    /// Choice field: Allow multiple selection.
    pub const CHOICE_MULTI_SELECT: FormFieldFlags =
        FormFieldFlags(FPDF_FORMFLAG_CHOICE_MULTI_SELECT);

    /// Check if a flag is set.
    pub fn contains(&self, flag: FormFieldFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Check if field is read-only.
    pub fn is_read_only(&self) -> bool {
        self.contains(Self::READ_ONLY)
    }

    /// Check if field is required.
    pub fn is_required(&self) -> bool {
        self.contains(Self::REQUIRED)
    }

    /// Check if this is a password field.
    pub fn is_password(&self) -> bool {
        self.contains(Self::TEXT_PASSWORD)
    }

    /// Check if this is a multi-line text field.
    pub fn is_multiline(&self) -> bool {
        self.contains(Self::TEXT_MULTILINE)
    }

    /// Check if this is an editable combo box.
    pub fn is_editable_combo(&self) -> bool {
        self.contains(Self::CHOICE_COMBO) && self.contains(Self::CHOICE_EDIT)
    }

    /// Check if multi-selection is allowed.
    pub fn allows_multi_select(&self) -> bool {
        self.contains(Self::CHOICE_MULTI_SELECT)
    }
}

/// An option in a choice field (combobox or listbox).
#[derive(Debug, Clone)]
pub struct FormFieldOption {
    /// The display label of the option.
    pub label: String,
    /// Index of this option in the options list.
    pub index: i32,
    /// Whether this option is currently selected.
    pub selected: bool,
}

/// A form field extracted from a widget annotation.
///
/// Form fields are interactive elements like text boxes, checkboxes, and dropdowns.
#[derive(Debug, Clone)]
pub struct PdfFormField {
    /// Field name (fully qualified)
    pub name: String,
    /// Alternate name (user-friendly name)
    pub alternate_name: Option<String>,
    /// Field type
    pub field_type: PdfFormFieldType,
    /// Current value (for text fields, checkboxes, etc.)
    pub value: String,
    /// Export value (for checkboxes and radio buttons)
    pub export_value: Option<String>,
    /// Field flags (read-only, required, etc.)
    pub flags: FormFieldFlags,
    /// Options for choice fields (combobox/listbox)
    pub options: Vec<FormFieldOption>,
    /// Number of controls (for radio button groups)
    pub control_count: i32,
    /// Control index within group
    pub control_index: i32,
    /// Font size (0 means auto-sized)
    pub font_size: Option<f32>,
    /// Whether this checkbox/radio is checked
    pub checked: bool,
    /// Bounding rectangle (left, top, right, bottom)
    pub rect: Option<(f32, f32, f32, f32)>,
}

impl PdfFormField {
    /// Check if this field is read-only.
    pub fn is_read_only(&self) -> bool {
        self.flags.is_read_only()
    }

    /// Check if this field is required.
    pub fn is_required(&self) -> bool {
        self.flags.is_required()
    }

    /// Check if this is a password field.
    pub fn is_password(&self) -> bool {
        self.flags.is_password()
    }

    /// Check if this is a multi-line text field.
    pub fn is_multiline(&self) -> bool {
        self.flags.is_multiline()
    }

    /// Get the selected options (for choice fields with multi-select).
    pub fn selected_options(&self) -> Vec<&FormFieldOption> {
        self.options.iter().filter(|o| o.selected).collect()
    }
}

/// Extract form field information from a widget annotation.
///
/// Widget annotations represent form fields in PDF.
pub fn extract_form_field(
    form_handle: FPDF_FORMHANDLE,
    annot: FPDF_ANNOTATION,
) -> Option<PdfFormField> {
    if form_handle.is_null() || annot.is_null() {
        return None;
    }

    // Get field type
    let field_type_raw = unsafe { FPDFAnnot_GetFormFieldType(form_handle, annot) };
    if field_type_raw < 0 {
        return None;
    }
    let field_type = PdfFormFieldType::from_raw(field_type_raw);

    // Get field name
    let name = get_annot_form_field_string(form_handle, annot, |h, a, buf, len| unsafe {
        FPDFAnnot_GetFormFieldName(h, a, buf, len)
    })?;

    // Get alternate name
    let alternate_name = get_annot_form_field_string(form_handle, annot, |h, a, buf, len| unsafe {
        FPDFAnnot_GetFormFieldAlternateName(h, a, buf, len)
    });

    // Get field value
    let value = get_annot_form_field_string(form_handle, annot, |h, a, buf, len| unsafe {
        FPDFAnnot_GetFormFieldValue(h, a, buf, len)
    })
    .unwrap_or_default();

    // Get export value (for checkboxes/radio buttons)
    let export_value = get_annot_form_field_string(form_handle, annot, |h, a, buf, len| unsafe {
        FPDFAnnot_GetFormFieldExportValue(h, a, buf, len)
    });

    // Get field flags
    let flags_raw = unsafe { FPDFAnnot_GetFormFieldFlags(form_handle, annot) };
    let flags = FormFieldFlags(flags_raw as u32);

    // Get control count and index
    let control_count = unsafe { FPDFAnnot_GetFormControlCount(form_handle, annot) };
    let control_index = unsafe { FPDFAnnot_GetFormControlIndex(form_handle, annot) };

    // Get font size
    let font_size = {
        let mut size: f32 = 0.0;
        if unsafe { FPDFAnnot_GetFontSize(form_handle, annot, &mut size) } != 0 {
            Some(size)
        } else {
            None
        }
    };

    // Check if checkbox/radio is checked
    let checked = unsafe { FPDFAnnot_IsChecked(form_handle, annot) } != 0;

    // Get options for choice fields
    let options = if field_type.is_choice() {
        extract_form_field_options(form_handle, annot)
    } else {
        Vec::new()
    };

    // Get rectangle
    let rect = {
        let mut r = FS_RECTF {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        };
        if unsafe { FPDFAnnot_GetRect(annot, &mut r) } != 0 {
            Some((r.left, r.top, r.right, r.bottom))
        } else {
            None
        }
    };

    Some(PdfFormField {
        name,
        alternate_name,
        field_type,
        value,
        export_value,
        flags,
        options,
        control_count,
        control_index,
        font_size,
        checked,
        rect,
    })
}

/// Extract options from a choice field (combobox or listbox).
fn extract_form_field_options(
    form_handle: FPDF_FORMHANDLE,
    annot: FPDF_ANNOTATION,
) -> Vec<FormFieldOption> {
    let option_count = unsafe { FPDFAnnot_GetOptionCount(form_handle, annot) };
    if option_count <= 0 {
        return Vec::new();
    }

    let mut options = Vec::with_capacity(option_count as usize);

    for i in 0..option_count {
        // Get option label
        let label = get_option_label(form_handle, annot, i).unwrap_or_default();

        // Check if option is selected
        let selected = unsafe { FPDFAnnot_IsOptionSelected(form_handle, annot, i) } != 0;

        options.push(FormFieldOption {
            label,
            index: i,
            selected,
        });
    }

    options
}

/// Helper to get an option label.
fn get_option_label(
    form_handle: FPDF_FORMHANDLE,
    annot: FPDF_ANNOTATION,
    index: i32,
) -> Option<String> {
    // Get required buffer length
    let len =
        unsafe { FPDFAnnot_GetOptionLabel(form_handle, annot, index, std::ptr::null_mut(), 0) };
    if len == 0 {
        return None;
    }

    // Allocate buffer
    let mut buffer: Vec<u16> = vec![0; (len / 2 + 1) as usize];
    let actual_len =
        unsafe { FPDFAnnot_GetOptionLabel(form_handle, annot, index, buffer.as_mut_ptr(), len) };
    if actual_len == 0 {
        return None;
    }

    // Convert UTF-16LE to String
    let chars = (actual_len / 2) as usize;
    let trimmed: Vec<u16> = buffer[..chars]
        .iter()
        .copied()
        .take_while(|&c| c != 0)
        .collect();

    String::from_utf16(&trimmed).ok()
}

/// Helper to get a string value from form field API.
fn get_annot_form_field_string<F>(
    form_handle: FPDF_FORMHANDLE,
    annot: FPDF_ANNOTATION,
    get_fn: F,
) -> Option<String>
where
    F: Fn(FPDF_FORMHANDLE, FPDF_ANNOTATION, *mut u16, u64) -> u64,
{
    // Get required buffer length
    let len = get_fn(form_handle, annot, std::ptr::null_mut(), 0);
    if len == 0 {
        return None;
    }

    // Allocate buffer
    let mut buffer: Vec<u16> = vec![0; (len / 2 + 1) as usize];
    let actual_len = get_fn(form_handle, annot, buffer.as_mut_ptr(), len);
    if actual_len == 0 {
        return None;
    }

    // Convert UTF-16LE to String
    let chars = (actual_len / 2) as usize;
    let trimmed: Vec<u16> = buffer[..chars]
        .iter()
        .copied()
        .take_while(|&c| c != 0)
        .collect();

    String::from_utf16(&trimmed).ok()
}

/// Iterator over form fields on a page.
pub struct PdfPageFormFields<'a> {
    form_handle: FPDF_FORMHANDLE,
    page_handle: FPDF_PAGE,
    annot_count: i32,
    current: i32,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> PdfPageFormFields<'a> {
    pub(crate) fn new(form_handle: FPDF_FORMHANDLE, page_handle: FPDF_PAGE) -> Self {
        let annot_count = unsafe { FPDFPage_GetAnnotCount(page_handle) };
        Self {
            form_handle,
            page_handle,
            annot_count,
            current: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for PdfPageFormFields<'a> {
    type Item = PdfFormField;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.annot_count {
            let annot = unsafe { FPDFPage_GetAnnot(self.page_handle, self.current) };
            self.current += 1;

            if annot.is_null() {
                continue;
            }

            // Check if it's a widget annotation
            let subtype = unsafe { FPDFAnnot_GetSubtype(annot) };
            if subtype as u32 != FPDF_ANNOT_WIDGET {
                unsafe { FPDFPage_CloseAnnot(annot) };
                continue;
            }

            // Extract form field info
            let field = extract_form_field(self.form_handle, annot);
            unsafe { FPDFPage_CloseAnnot(annot) };

            if let Some(f) = field {
                return Some(f);
            }
        }
        None
    }
}

/// Error type for form field operations
#[derive(Debug, Clone)]
pub enum FormError {
    /// The field is read-only and cannot be modified
    ReadOnly,
    /// The field type doesn't support this operation
    UnsupportedOperation(&'static str),
    /// Failed to set the field value
    SetValueFailed(String),
    /// Invalid index for choice field
    InvalidIndex(i32),
    /// Annotation not found
    AnnotationNotFound,
}

impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormError::ReadOnly => write!(f, "Field is read-only"),
            FormError::UnsupportedOperation(op) => write!(f, "Unsupported operation: {}", op),
            FormError::SetValueFailed(msg) => write!(f, "Failed to set value: {}", msg),
            FormError::InvalidIndex(idx) => write!(f, "Invalid option index: {}", idx),
            FormError::AnnotationNotFound => write!(f, "Annotation not found"),
        }
    }
}

impl std::error::Error for FormError {}

/// Result type for form operations
pub type FormResult<T> = std::result::Result<T, FormError>;

/// A mutable form field editor that allows setting form field values.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("form.pdf", None)?;
/// let page = doc.page(0)?;
///
/// // Get a mutable form field editor
/// if let Some(mut editor) = page.form_field_editor(0) {
///     // Set text field value
///     let _ = editor.set_text_value("New value");
///
///     // Or set a choice field selection
///     let _ = editor.set_option_selected(0, true);
/// }
///
/// // Save the modified document
/// doc.save_to_file("modified.pdf", None)?;
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
pub struct PdfFormFieldEditor {
    form_handle: FPDF_FORMHANDLE,
    page_handle: FPDF_PAGE,
    annot: FPDF_ANNOTATION,
    annot_index: i32,
    field_type: PdfFormFieldType,
    flags: FormFieldFlags,
    owns_annot: bool,
}

impl PdfFormFieldEditor {
    /// Create a new form field editor.
    ///
    /// # Safety
    ///
    /// The handles must be valid for the lifetime of this editor.
    pub(crate) fn new(
        form_handle: FPDF_FORMHANDLE,
        page_handle: FPDF_PAGE,
        annot_index: i32,
    ) -> Option<Self> {
        if form_handle.is_null() || page_handle.is_null() {
            return None;
        }

        let annot = unsafe { FPDFPage_GetAnnot(page_handle, annot_index) };
        if annot.is_null() {
            return None;
        }

        // Check if it's a widget annotation
        let subtype = unsafe { FPDFAnnot_GetSubtype(annot) };
        if subtype as u32 != FPDF_ANNOT_WIDGET {
            unsafe { FPDFPage_CloseAnnot(annot) };
            return None;
        }

        // Get field type
        let field_type_raw = unsafe { FPDFAnnot_GetFormFieldType(form_handle, annot) };
        if field_type_raw < 0 {
            unsafe { FPDFPage_CloseAnnot(annot) };
            return None;
        }
        let field_type = PdfFormFieldType::from_raw(field_type_raw);

        // Get flags
        let flags_raw = unsafe { FPDFAnnot_GetFormFieldFlags(form_handle, annot) };
        let flags = FormFieldFlags(flags_raw as u32);

        Some(Self {
            form_handle,
            page_handle,
            annot,
            annot_index,
            field_type,
            flags,
            owns_annot: true,
        })
    }

    /// Get the field type.
    pub fn field_type(&self) -> PdfFormFieldType {
        self.field_type
    }

    /// Check if the field is read-only.
    pub fn is_read_only(&self) -> bool {
        self.flags.is_read_only()
    }

    /// Get the current field value as a string.
    pub fn value(&self) -> Option<String> {
        get_annot_form_field_string(self.form_handle, self.annot, |h, a, buf, len| unsafe {
            FPDFAnnot_GetFormFieldValue(h, a, buf, len)
        })
    }

    /// Get the field name.
    pub fn name(&self) -> Option<String> {
        get_annot_form_field_string(self.form_handle, self.annot, |h, a, buf, len| unsafe {
            FPDFAnnot_GetFormFieldName(h, a, buf, len)
        })
    }

    /// Set the text value of a text field.
    ///
    /// This works for text fields. For other field types, use the appropriate method.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The field is read-only
    /// - The field is not a text field
    /// - The value could not be set
    pub fn set_text_value(&mut self, value: &str) -> FormResult<()> {
        if self.flags.is_read_only() {
            return Err(FormError::ReadOnly);
        }

        if !self.field_type.is_text() && !matches!(self.field_type, PdfFormFieldType::ComboBox) {
            return Err(FormError::UnsupportedOperation(
                "set_text_value only works on text fields and editable combo boxes",
            ));
        }

        // For text fields, we need to:
        // 1. Focus the annotation
        // 2. Select all text
        // 3. Replace with new text

        // Focus the annotation
        let focus_result = unsafe { FORM_SetFocusedAnnot(self.form_handle, self.annot) };
        if focus_result == 0 {
            // Focus failed, try setting value directly via annotation dictionary
            return self.set_value_via_dictionary(value);
        }

        // Select all text
        unsafe { FORM_SelectAllText(self.form_handle, self.page_handle) };

        // Convert value to UTF-16LE with null terminator
        let utf16: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();

        // Replace selection
        unsafe {
            FORM_ReplaceSelection(self.form_handle, self.page_handle, utf16.as_ptr());
        }

        // Remove focus to finalize
        unsafe { FORM_ForceToKillFocus(self.form_handle) };

        Ok(())
    }

    /// Set the value directly via the annotation dictionary.
    ///
    /// This is a fallback method when FORM_* APIs don't work.
    fn set_value_via_dictionary(&mut self, value: &str) -> FormResult<()> {
        // The "V" key holds the field value
        let key = b"V\0";
        let utf16: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();

        let result = unsafe {
            FPDFAnnot_SetStringValue(
                self.annot,
                key.as_ptr() as *const ::std::os::raw::c_char,
                utf16.as_ptr(),
            )
        };

        if result == 0 {
            Err(FormError::SetValueFailed(
                "FPDFAnnot_SetStringValue failed".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Set whether a checkbox or radio button is checked.
    ///
    /// # Arguments
    ///
    /// * `checked` - Whether the checkbox/radio should be checked
    ///
    /// # Errors
    ///
    /// Returns an error if the field is not a checkbox or radio button.
    pub fn set_checked(&mut self, checked: bool) -> FormResult<()> {
        if self.flags.is_read_only() {
            return Err(FormError::ReadOnly);
        }

        if !self.field_type.is_button() || self.field_type == PdfFormFieldType::PushButton {
            return Err(FormError::UnsupportedOperation(
                "set_checked only works on checkboxes and radio buttons",
            ));
        }

        // Get the export value for this checkbox/radio
        let export_value = if checked {
            // Get the export value (e.g., "Yes", "On", or a custom value)
            get_annot_form_field_string(self.form_handle, self.annot, |h, a, buf, len| unsafe {
                FPDFAnnot_GetFormFieldExportValue(h, a, buf, len)
            })
            .unwrap_or_else(|| "Yes".to_string())
        } else {
            // "Off" is the standard value for unchecked state
            "Off".to_string()
        };

        // Set the value in the annotation dictionary
        self.set_value_via_dictionary(&export_value)
    }

    /// Set whether an option in a choice field is selected.
    ///
    /// # Arguments
    ///
    /// * `index` - The option index (0-based)
    /// * `selected` - Whether the option should be selected
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The field is not a choice field (combo box or list box)
    /// - The index is out of range
    pub fn set_option_selected(&mut self, index: i32, selected: bool) -> FormResult<()> {
        if self.flags.is_read_only() {
            return Err(FormError::ReadOnly);
        }

        if !self.field_type.is_choice() {
            return Err(FormError::UnsupportedOperation(
                "set_option_selected only works on combo boxes and list boxes",
            ));
        }

        // Check index is valid
        let option_count = unsafe { FPDFAnnot_GetOptionCount(self.form_handle, self.annot) };
        if index < 0 || index >= option_count {
            return Err(FormError::InvalidIndex(index));
        }

        let result = unsafe {
            FORM_SetIndexSelected(
                self.form_handle,
                self.page_handle,
                index,
                if selected { 1 } else { 0 },
            )
        };

        if result == 0 {
            // Fallback: try to set value directly
            if selected {
                // Get the option label to set as value
                if let Some(label) = get_option_label(self.form_handle, self.annot, index) {
                    return self.set_value_via_dictionary(&label);
                }
            }
            Err(FormError::SetValueFailed(
                "FORM_SetIndexSelected failed".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Select an option by its label.
    ///
    /// # Arguments
    ///
    /// * `label` - The option label to select
    ///
    /// # Errors
    ///
    /// Returns an error if the option is not found or the field is not a choice field.
    pub fn select_option_by_label(&mut self, label: &str) -> FormResult<()> {
        if !self.field_type.is_choice() {
            return Err(FormError::UnsupportedOperation(
                "select_option_by_label only works on choice fields",
            ));
        }

        let option_count = unsafe { FPDFAnnot_GetOptionCount(self.form_handle, self.annot) };

        for i in 0..option_count {
            if let Some(opt_label) = get_option_label(self.form_handle, self.annot, i) {
                if opt_label == label {
                    return self.set_option_selected(i, true);
                }
            }
        }

        Err(FormError::SetValueFailed(format!(
            "Option '{}' not found",
            label
        )))
    }

    /// Clear all selections in a multi-select list box.
    ///
    /// # Errors
    ///
    /// Returns an error if the field is not a list box with multi-select enabled.
    pub fn clear_selections(&mut self) -> FormResult<()> {
        if !self.field_type.is_choice() {
            return Err(FormError::UnsupportedOperation(
                "clear_selections only works on choice fields",
            ));
        }

        let option_count = unsafe { FPDFAnnot_GetOptionCount(self.form_handle, self.annot) };

        for i in 0..option_count {
            let is_selected =
                unsafe { FPDFAnnot_IsOptionSelected(self.form_handle, self.annot, i) };
            if is_selected != 0 {
                let _ = self.set_option_selected(i, false);
            }
        }

        Ok(())
    }

    /// Get the number of options (for choice fields).
    pub fn option_count(&self) -> i32 {
        if self.field_type.is_choice() {
            unsafe { FPDFAnnot_GetOptionCount(self.form_handle, self.annot) }
        } else {
            0
        }
    }

    /// Check if an option is selected (for choice fields).
    pub fn is_option_selected(&self, index: i32) -> bool {
        if !self.field_type.is_choice() {
            return false;
        }
        let result = unsafe { FPDFAnnot_IsOptionSelected(self.form_handle, self.annot, index) };
        result != 0
    }

    /// Check if the checkbox/radio is currently checked.
    pub fn is_checked(&self) -> bool {
        let result = unsafe { FPDFAnnot_IsChecked(self.form_handle, self.annot) };
        result != 0
    }

    /// Get the annotation index.
    pub fn annotation_index(&self) -> i32 {
        self.annot_index
    }
}

impl Drop for PdfFormFieldEditor {
    fn drop(&mut self) {
        if self.owns_annot && !self.annot.is_null() {
            unsafe {
                FPDFPage_CloseAnnot(self.annot);
            }
        }
    }
}

/// Iterator over mutable form field editors on a page.
pub struct PdfPageFormFieldEditors {
    form_handle: FPDF_FORMHANDLE,
    page_handle: FPDF_PAGE,
    annot_count: i32,
    current: i32,
}

impl PdfPageFormFieldEditors {
    pub(crate) fn new(form_handle: FPDF_FORMHANDLE, page_handle: FPDF_PAGE) -> Self {
        let annot_count = unsafe { FPDFPage_GetAnnotCount(page_handle) };
        Self {
            form_handle,
            page_handle,
            annot_count,
            current: 0,
        }
    }
}

impl Iterator for PdfPageFormFieldEditors {
    type Item = PdfFormFieldEditor;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.annot_count {
            let idx = self.current;
            self.current += 1;

            if let Some(editor) = PdfFormFieldEditor::new(self.form_handle, self.page_handle, idx) {
                return Some(editor);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_error_display() {
        let err = FormError::ReadOnly;
        assert_eq!(format!("{}", err), "Field is read-only");

        let err = FormError::InvalidIndex(5);
        assert_eq!(format!("{}", err), "Invalid option index: 5");
    }

    #[test]
    fn test_form_field_type_from_raw() {
        assert_eq!(
            PdfFormFieldType::from_raw(FPDF_FORMFIELD_TEXTFIELD as i32),
            PdfFormFieldType::TextField
        );
        assert_eq!(
            PdfFormFieldType::from_raw(FPDF_FORMFIELD_CHECKBOX as i32),
            PdfFormFieldType::CheckBox
        );
        assert_eq!(PdfFormFieldType::from_raw(-1), PdfFormFieldType::Unknown);
    }

    #[test]
    fn test_form_field_type_categories() {
        assert!(PdfFormFieldType::PushButton.is_button());
        assert!(PdfFormFieldType::CheckBox.is_button());
        assert!(PdfFormFieldType::RadioButton.is_button());
        assert!(!PdfFormFieldType::TextField.is_button());

        assert!(PdfFormFieldType::ComboBox.is_choice());
        assert!(PdfFormFieldType::ListBox.is_choice());
        assert!(!PdfFormFieldType::TextField.is_choice());

        assert!(PdfFormFieldType::TextField.is_text());
        assert!(!PdfFormFieldType::ComboBox.is_text());
    }

    #[test]
    fn test_form_type_from_raw() {
        assert_eq!(PdfFormType::from_raw(0), PdfFormType::None);
        assert_eq!(PdfFormType::from_raw(1), PdfFormType::AcroForm);
        assert_eq!(PdfFormType::from_raw(2), PdfFormType::XfaFull);
    }

    #[test]
    fn test_form_field_flags() {
        let flags = FormFieldFlags::READ_ONLY;
        assert!(flags.is_read_only());
        assert!(!flags.is_required());

        let required = FormFieldFlags::REQUIRED;
        assert!(required.is_required());
        assert!(!required.is_read_only());

        let combined = FormFieldFlags(FormFieldFlags::READ_ONLY.0 | FormFieldFlags::REQUIRED.0);
        assert!(combined.is_read_only());
        assert!(combined.is_required());
    }

    #[test]
    fn test_form_field_flags_text() {
        let multiline = FormFieldFlags::TEXT_MULTILINE;
        assert!(multiline.is_multiline());
        assert!(!multiline.is_password());

        let password = FormFieldFlags::TEXT_PASSWORD;
        assert!(password.is_password());
        assert!(!password.is_multiline());
    }

    #[test]
    fn test_form_field_flags_choice() {
        let combo_edit =
            FormFieldFlags(FormFieldFlags::CHOICE_COMBO.0 | FormFieldFlags::CHOICE_EDIT.0);
        assert!(combo_edit.is_editable_combo());

        let multi = FormFieldFlags::CHOICE_MULTI_SELECT;
        assert!(multi.allows_multi_select());
    }

    #[test]
    fn test_form_field_option() {
        let option = FormFieldOption {
            label: "Test Option".to_string(),
            index: 0,
            selected: true,
        };
        assert_eq!(option.label, "Test Option");
        assert_eq!(option.index, 0);
        assert!(option.selected);
    }
}
