//! VCF/vCard (Contact Cards) parser
//!
//! Parses RFC 6350 vCard contact files.
//! Supports vCard 2.1, 3.0, and 4.0 formats.
//!
//! This is a simple parser that extracts the most common properties.
//! For full RFC 6350 compliance, consider using a dedicated vCard library.

use crate::error::{EmailError, Result};
use std::collections::HashMap;
use std::fmt::Write;

/// Email address with optional type (work, home, etc.) and preference flag
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TypedEmail {
    /// Email address string
    pub address: String,
    /// Type label (e.g., "work", "home")
    pub type_label: Option<String>,
    /// Whether this is the preferred email (PREF=1)
    pub preferred: bool,
}

/// Phone number with optional type (work, cell, fax, etc.)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TypedPhone {
    /// Phone number string
    pub number: String,
    /// Type label (e.g., "work", "cell", "fax")
    pub type_label: Option<String>,
}

/// Parsed vCard contact
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Contact {
    /// Full name
    pub name: String,
    /// vCard version (e.g., "3.0", "4.0")
    pub version: Option<String>,
    /// Email addresses with type information
    pub emails: Vec<TypedEmail>,
    /// Phone numbers with type information
    pub phones: Vec<TypedPhone>,
    /// Organization
    pub organization: Option<String>,
    /// Title/Role
    pub title: Option<String>,
    /// Additional properties
    pub properties: HashMap<String, String>,
}

/// Parse a VCF file from bytes (may contain multiple vCards)
///
/// # Errors
///
/// Returns an error if:
/// - The content is not valid UTF-8
/// - No vCards are found in the file
#[must_use = "this function returns parsed contacts that should be processed"]
pub fn parse_vcf(content: &[u8]) -> Result<Vec<Contact>> {
    let content_str = std::str::from_utf8(content)
        .map_err(|e| EmailError::VCardError(format!("Invalid UTF-8: {e}")))?;

    let contacts = parse_vcards_simple(content_str);

    if contacts.is_empty() {
        return Err(EmailError::VCardError(
            "No vCards found in file".to_string(),
        ));
    }

    Ok(contacts)
}

/// Type alias for vCard property map
/// Maps property name to list of (value, parameters)
type VCardProperties = HashMap<String, Vec<(String, Option<HashMap<String, String>>)>>;

/// Simple vCard parser
fn parse_vcards_simple(content: &str) -> Vec<Contact> {
    let mut contacts = Vec::new();
    let mut current_vcard: Option<VCardProperties> = None;

    for line in content.lines() {
        let line = line.trim();

        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            current_vcard = Some(HashMap::new());
        } else if line.eq_ignore_ascii_case("END:VCARD") {
            if let Some(vcard) = current_vcard.take() {
                contacts.push(extract_contact(&vcard));
            }
        } else if let Some(ref mut vcard) = current_vcard {
            // Parse property line (KEY:VALUE or KEY;PARAMS:VALUE)
            if let Some((key, value)) = line.split_once(':') {
                // Extract parameters (TYPE, PREF, VALUE, etc.)
                let key_parts: Vec<&str> = key.split(';').collect();
                let key_clean = key_parts[0].to_uppercase();

                // Build parameter map for this property
                let mut params = HashMap::new();
                for part in key_parts.iter().skip(1) {
                    if let Some((param_key, param_val)) = part.split_once('=') {
                        params.insert(param_key.to_uppercase(), param_val.to_string());
                    }
                }

                // Store value with all parameters
                vcard
                    .entry(key_clean)
                    .or_insert_with(Vec::new)
                    .push((value.to_string(), Some(params)));
            }
        }
    }

    contacts
}

/// Format vCard ADR field into human-readable address with labeled fields
///
/// ADR structure (RFC 6350): PO box; extended address; street; city; region; postal code; country
#[inline]
fn format_address(parts: &[&str]) -> String {
    let mut address_lines: Vec<String> = Vec::new();

    // PO box (index 0)
    if !parts.is_empty() && !parts[0].is_empty() {
        let first_field = parts[0];
        if first_field
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace())
        {
            // Looks like a PO Box number
            address_lines.push(format!("PO Box: {first_field}"));
        } else {
            // It's something else (e.g., "Suite 400")
            address_lines.push(format!("Suite: {first_field}"));
        }
    }

    // Extended address (index 1)
    if parts.len() > 1 && !parts[1].is_empty() {
        address_lines.push(format!("Building: {}", parts[1]));
    }

    // Street (index 2)
    if parts.len() > 2 && !parts[2].is_empty() {
        address_lines.push(format!("Street: {}", parts[2]));
    }

    // City (index 3)
    if parts.len() > 3 && !parts[3].is_empty() {
        address_lines.push(format!("City: {}", parts[3]));
    }

    // Region/State (index 4)
    if parts.len() > 4 && !parts[4].is_empty() {
        address_lines.push(format!("State: {}", parts[4]));
    }

    // Postal code (index 5)
    if parts.len() > 5 && !parts[5].is_empty() {
        address_lines.push(format!("Postal Code: {}", parts[5]));
    }

    // Country (index 6)
    if parts.len() > 6 && !parts[6].is_empty() {
        address_lines.push(format!("Country: {}", parts[6]));
    }

    address_lines.join("\n")
}

/// Extract Contact from parsed vCard properties
#[allow(clippy::too_many_lines)] // Complex vCard extraction - keeping together for clarity
fn extract_contact(vcard: &VCardProperties) -> Contact {
    // Extract full name
    let name = vcard
        .get("FN")
        .and_then(|v| v.first())
        .map_or_else(|| "(No Name)".to_string(), |(val, _)| val.clone());

    // Extract version
    let version = vcard
        .get("VERSION")
        .and_then(|v| v.first())
        .map(|(val, _)| val.clone());

    // Extract email addresses with type information
    let emails = vcard
        .get("EMAIL")
        .map(|items| {
            items
                .iter()
                .map(|(val, params)| {
                    let type_label = params
                        .as_ref()
                        .and_then(|p| p.get("TYPE"))
                        .map(|t| t.to_lowercase());
                    let preferred = params
                        .as_ref()
                        .and_then(|p| p.get("PREF"))
                        .is_some_and(|p| p == "1");
                    TypedEmail {
                        address: val.clone(),
                        type_label,
                        preferred,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Extract phone numbers with type information
    let phones = vcard
        .get("TEL")
        .map(|items| {
            items
                .iter()
                .map(|(val, params)| {
                    let type_label = params
                        .as_ref()
                        .and_then(|p| p.get("TYPE"))
                        .map(|t| t.to_lowercase());
                    // Strip "tel:" prefix if present (from VALUE=uri)
                    let number = val
                        .strip_prefix("tel:")
                        .map_or_else(|| val.clone(), std::string::ToString::to_string);
                    TypedPhone { number, type_label }
                })
                .collect()
        })
        .unwrap_or_default();

    // Extract organization
    let organization = vcard
        .get("ORG")
        .and_then(|v| v.first())
        .map(|(val, _)| val.clone());

    // Extract title
    let title = vcard
        .get("TITLE")
        .and_then(|v| v.first())
        .map(|(val, _)| val.clone());

    // Extract additional properties
    let mut properties = HashMap::new();

    // N (structured name) - extract honorific suffix and other components if present
    if let Some(n_field) = vcard.get("N").and_then(|v| v.first()).map(|(val, _)| val) {
        let parts: Vec<&str> = n_field.split(';').collect();
        // N structure: Family;Given;Additional;Prefix;Suffix
        // Only add if there are interesting parts beyond what's in FN
        if parts.len() > 3 && !parts[3].is_empty() {
            // Prefix (e.g., "Dr.", "Prof.")
            properties.insert("honorific_prefix".to_string(), parts[3].to_string());
        }
        if parts.len() > 4 && !parts[4].is_empty() {
            // Suffix (e.g., "Ph.D.", "Jr.", "III")
            properties.insert("honorific_suffix".to_string(), parts[4].to_string());
        }
        if parts.len() > 2 && !parts[2].is_empty() {
            // Additional names (middle names)
            properties.insert("middle_name".to_string(), parts[2].to_string());
        }
    }

    // URL(s) - can have multiple, use TYPE parameter for better labeling
    if let Some(urls) = vcard.get("URL") {
        for (url_count, (url, params)) in urls.iter().enumerate() {
            // Default key based on URL count
            let default_key = || {
                if url_count == 0 {
                    "url".to_string()
                } else {
                    format!("url_{}", url_count + 1)
                }
            };
            // Use TYPE parameter if available (e.g., "linkedin", "work")
            let key = params
                .as_ref()
                .and_then(|p| p.get("TYPE"))
                .map_or_else(default_key, |url_type| {
                    format!("url_{}", url_type.to_lowercase())
                });
            properties.insert(key, url.clone());
        }
    }

    // NOTE
    if let Some(note) = vcard
        .get("NOTE")
        .and_then(|v| v.first())
        .map(|(val, _)| val)
    {
        properties.insert("note".to_string(), note.clone());
    }

    // ADR - Address (structured: PO;extended;street;city;region;postal;country)
    if let Some(addresses) = vcard.get("ADR") {
        for (i, (addr, _)) in addresses.iter().enumerate() {
            let key = if i == 0 {
                "address".to_string()
            } else {
                format!("address_{}", i + 1)
            };
            // Convert semicolon-separated to human-readable
            let parts: Vec<&str> = addr.split(';').collect();
            let readable = format_address(&parts);
            properties.insert(key, readable);
        }
    }

    // BDAY - Birthday
    if let Some(bday) = vcard
        .get("BDAY")
        .and_then(|v| v.first())
        .map(|(val, _)| val)
    {
        properties.insert("birthday".to_string(), bday.clone());
    }

    // LANG - Language(s)
    if let Some(langs) = vcard.get("LANG") {
        let lang_list = langs
            .iter()
            .map(|(val, _)| val.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        properties.insert("languages".to_string(), lang_list);
    }

    // GENDER
    if let Some(gender) = vcard
        .get("GENDER")
        .and_then(|v| v.first())
        .map(|(val, _)| val)
    {
        properties.insert("gender".to_string(), gender.clone());
    }

    // CATEGORIES
    if let Some(categories) = vcard
        .get("CATEGORIES")
        .and_then(|v| v.first())
        .map(|(val, _)| val)
    {
        properties.insert("categories".to_string(), categories.clone());
    }

    Contact {
        name,
        version,
        emails,
        phones,
        organization,
        title,
        properties,
    }
}

/// Convert contacts to markdown (preserving vCard structure for LLM quality)
#[must_use = "converts contacts to markdown format"]
pub fn vcf_to_markdown(contacts: &[Contact]) -> String {
    let mut output = String::new();

    // LLM Quality (N=2194): Explicitly indicate vCard format in title
    if contacts.len() == 1 {
        output.push_str("# vCard Contact\n\n");
    } else {
        let _ = writeln!(output, "# vCard Contacts ({} total)\n", contacts.len());
    }

    for (i, contact) in contacts.iter().enumerate() {
        if i > 0 {
            output.push_str("\n---\n\n");
        }

        // Name as header
        let _ = writeln!(output, "## {}\n", contact.name);

        // vCard version as separate section (LLM Quality N=2297)
        if let Some(version) = &contact.version {
            let _ = writeln!(output, "**vCard Version:** {version}\n");
        }

        // Organization and title (human-readable before vCard block)
        if let Some(title) = &contact.title {
            if let Some(org) = &contact.organization {
                let _ = writeln!(output, "{title} at {org}\n");
            } else {
                let _ = writeln!(output, "{title}\n");
            }
        } else if let Some(org) = &contact.organization {
            let _ = writeln!(output, "{org}\n");
        }

        // Preserve vCard structure in code block
        output.push_str("```vcard\n");
        output.push_str("BEGIN:VCARD\n");

        // vCard version (also in code block for completeness)
        if let Some(version) = &contact.version {
            let _ = writeln!(output, "VERSION:{version}");
        }

        // Full name
        let _ = writeln!(output, "FN:{}", contact.name);

        // Organization and title
        if let Some(org) = &contact.organization {
            let _ = writeln!(output, "ORG:{org}");
        }
        if let Some(title) = &contact.title {
            let _ = writeln!(output, "TITLE:{title}");
        }

        // Email addresses with types
        for email in &contact.emails {
            let line = email.type_label.as_ref().map_or_else(
                || format!("EMAIL:{}", email.address),
                |type_label| {
                    let pref = if email.preferred { ";PREF=1" } else { "" };
                    format!("EMAIL;TYPE={type_label}{pref}:{}", email.address)
                },
            );
            let _ = writeln!(output, "{line}");
        }

        // Phone numbers with types
        for phone in &contact.phones {
            let line = phone.type_label.as_ref().map_or_else(
                || format!("TEL:{}", phone.number),
                |type_label| format!("TEL;TYPE={type_label}:{}", phone.number),
            );
            let _ = writeln!(output, "{line}");
        }

        // Additional properties
        for (key, value) in &contact.properties {
            // Format property name in vCard style (uppercase, replace underscores)
            let vcard_key = key.to_uppercase().replace('_', "-");
            let _ = writeln!(output, "X-{vcard_key}:{value}");
        }

        output.push_str("END:VCARD\n");
        output.push_str("```\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_vcard() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:John Doe\r\n\
                    EMAIL:john@example.com\r\n\
                    TEL:+1-555-1234\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "John Doe");
        assert_eq!(contacts[0].emails.len(), 1);
        assert_eq!(contacts[0].emails[0].address, "john@example.com");
        assert_eq!(contacts[0].phones.len(), 1);
    }

    #[test]
    fn test_parse_multiple_vcards() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:John Doe\r\n\
                    EMAIL:john@example.com\r\n\
                    END:VCARD\r\n\
                    BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Jane Smith\r\n\
                    EMAIL:jane@example.com\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 2);
        assert_eq!(contacts[0].name, "John Doe");
        assert_eq!(contacts[1].name, "Jane Smith");
    }

    #[test]
    fn test_parse_empty_fails() {
        let result = parse_vcf(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse_vcf(b"Not a vCard");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_vcard_with_parameters() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:John Doe\r\n\
                    EMAIL;TYPE=work:john.work@example.com\r\n\
                    EMAIL;TYPE=home:john.home@example.com\r\n\
                    TEL;TYPE=cell:+1-555-1234\r\n\
                    ORG:Acme Corp\r\n\
                    TITLE:Engineer\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "John Doe");
        assert_eq!(contacts[0].emails.len(), 2);
        assert_eq!(contacts[0].phones.len(), 1);
        assert_eq!(contacts[0].organization, Some("Acme Corp".to_string()));
        assert_eq!(contacts[0].title, Some("Engineer".to_string()));
    }

    #[test]
    fn test_vcf_to_markdown_preserves_structure() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:John Doe\r\n\
                    EMAIL:john@example.com\r\n\
                    TEL:+1-555-1234\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        let markdown = vcf_to_markdown(&contacts);

        // Verify vCard structure is preserved
        assert!(
            markdown.contains("BEGIN:VCARD"),
            "Markdown must contain BEGIN:VCARD marker"
        );
        assert!(
            markdown.contains("END:VCARD"),
            "Markdown must contain END:VCARD marker"
        );
        assert!(
            markdown.contains("```vcard"),
            "Markdown must use vcard code block"
        );
        assert!(
            markdown.contains("VERSION:3.0"),
            "Markdown must preserve VERSION field"
        );
        assert!(
            markdown.contains("FN:John Doe"),
            "Markdown must preserve FN field"
        );
        assert!(
            markdown.contains("EMAIL:john@example.com"),
            "Markdown must preserve EMAIL field"
        );
        assert!(
            markdown.contains("TEL:+1-555-1234"),
            "Markdown must preserve TEL field"
        );
    }

    /// Test vCard 4.0 format
    #[test]
    fn test_parse_vcard_v4() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:4.0\r\n\
                    FN:Alice Johnson\r\n\
                    EMAIL:alice@example.com\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].version, Some("4.0".to_string()));
        assert_eq!(contacts[0].name, "Alice Johnson");
    }

    /// Test vCard 2.1 format (older, still common)
    #[test]
    fn test_parse_vcard_v21() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:2.1\r\n\
                    FN:Bob Wilson\r\n\
                    TEL:555-1234\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].version, Some("2.1".to_string()));
    }

    /// Test structured name (N field) parsing
    #[test]
    fn test_parse_vcard_with_structured_name() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Dr. John Michael Doe Jr.\r\n\
                    N:Doe;John;Michael;Dr.;Jr.\r\n\
                    EMAIL:john@example.com\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "Dr. John Michael Doe Jr.");
        assert_eq!(
            contacts[0].properties.get("honorific_prefix"),
            Some(&"Dr.".to_string())
        );
        assert_eq!(
            contacts[0].properties.get("honorific_suffix"),
            Some(&"Jr.".to_string())
        );
        assert_eq!(
            contacts[0].properties.get("middle_name"),
            Some(&"Michael".to_string())
        );
    }

    /// Test address parsing
    #[test]
    fn test_parse_vcard_with_address() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Jane Doe\r\n\
                    ADR:;;123 Main St;Anytown;CA;12345;USA\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        // Address should be formatted into properties
        assert!(contacts[0].properties.contains_key("address"));
    }

    /// Test birthday parsing
    #[test]
    fn test_parse_vcard_with_birthday() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Test User\r\n\
                    BDAY:1990-01-15\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(
            contacts[0].properties.get("birthday"),
            Some(&"1990-01-15".to_string())
        );
    }

    /// Test URL parsing
    #[test]
    fn test_parse_vcard_with_url() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Web Developer\r\n\
                    URL:https://example.com\r\n\
                    URL;TYPE=linkedin:https://linkedin.com/in/webdev\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert!(contacts[0].properties.contains_key("url"));
    }

    /// Test note parsing
    #[test]
    fn test_parse_vcard_with_note() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Contact With Note\r\n\
                    NOTE:This is a note about the contact.\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(
            contacts[0].properties.get("note"),
            Some(&"This is a note about the contact.".to_string())
        );
    }

    /// Test categories parsing
    #[test]
    fn test_parse_vcard_with_categories() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Categorized Contact\r\n\
                    CATEGORIES:Friends,Work,VIP\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(
            contacts[0].properties.get("categories"),
            Some(&"Friends,Work,VIP".to_string())
        );
    }

    /// Test preferred email (PREF parameter)
    #[test]
    fn test_parse_vcard_with_preferred_email() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Multi-Email Contact\r\n\
                    EMAIL;TYPE=work:work@example.com\r\n\
                    EMAIL;TYPE=home;PREF=1:home@example.com\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].emails.len(), 2);
        // Find the preferred email
        let preferred = contacts[0].emails.iter().find(|e| e.preferred);
        assert!(preferred.is_some(), "Should have a preferred email");
        assert_eq!(preferred.unwrap().address, "home@example.com");
    }

    /// Test multiple phone types
    #[test]
    fn test_parse_vcard_with_multiple_phones() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:3.0\r\n\
                    FN:Multi-Phone Contact\r\n\
                    TEL;TYPE=work:+1-555-1111\r\n\
                    TEL;TYPE=cell:+1-555-2222\r\n\
                    TEL;TYPE=fax:+1-555-3333\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].phones.len(), 3);
        // Verify types
        let types: Vec<_> = contacts[0]
            .phones
            .iter()
            .filter_map(|p| p.type_label.clone())
            .collect();
        assert!(types.contains(&"work".to_string()));
        assert!(types.contains(&"cell".to_string()));
        assert!(types.contains(&"fax".to_string()));
    }

    /// Test comprehensive contact with all fields
    #[test]
    fn test_parse_comprehensive_vcard() {
        let vcf = b"BEGIN:VCARD\r\n\
                    VERSION:4.0\r\n\
                    FN:Dr. Complete Contact III\r\n\
                    N:Contact;Complete;Middle;Dr.;III\r\n\
                    ORG:Test Corporation\r\n\
                    TITLE:Chief Testing Officer\r\n\
                    EMAIL;TYPE=work;PREF=1:work@test.com\r\n\
                    EMAIL;TYPE=home:home@test.com\r\n\
                    TEL;TYPE=cell:+1-555-0000\r\n\
                    ADR:;;1 Test St;Testville;TS;00000;Testland\r\n\
                    BDAY:1985-06-15\r\n\
                    URL:https://test.com\r\n\
                    NOTE:Comprehensive test contact\r\n\
                    CATEGORIES:Test,Comprehensive\r\n\
                    END:VCARD\r\n";

        let contacts = parse_vcf(vcf).unwrap();
        assert_eq!(contacts.len(), 1);
        let contact = &contacts[0];

        assert_eq!(contact.name, "Dr. Complete Contact III");
        assert_eq!(contact.version, Some("4.0".to_string()));
        assert_eq!(contact.organization, Some("Test Corporation".to_string()));
        assert_eq!(contact.title, Some("Chief Testing Officer".to_string()));
        assert_eq!(contact.emails.len(), 2);
        assert_eq!(contact.phones.len(), 1);
        assert!(contact.properties.contains_key("address"));
        assert!(contact.properties.contains_key("birthday"));
        assert!(contact.properties.contains_key("url"));
        assert!(contact.properties.contains_key("note"));
        assert!(contact.properties.contains_key("categories"));
    }
}
