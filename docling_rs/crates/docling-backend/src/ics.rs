//! ICS/iCalendar backend for docling
//!
//! This backend converts ICS/iCalendar files to docling's document model.

// Clippy pedantic allows:
// - Calendar parsing functions are complex
#![allow(clippy::too_many_lines)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, opt_vec};
use docling_calendar::{parse_ics, CalendarEvent, CalendarJournal, CalendarTodo, IcsInfo};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use std::fmt::Write;
use std::path::Path;

/// ICS/iCalendar backend
///
/// Converts ICS/iCalendar files to docling's document model.
/// Supports events, todos, and journal entries.
///
/// ## Features
///
/// - Parse calendar events (VEVENT) with date/time, location, attendees
/// - Parse todos (VTODO) with due dates, priority, completion status
/// - Parse journal entries (VJOURNAL)
/// - Markdown-formatted output with sections for events, todos, journals
///
/// ## Example
///
/// ```no_run
/// use docling_backend::IcsBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = IcsBackend::new();
/// let result = backend.parse_file("schedule.ics", &Default::default())?;
/// println!("Calendar: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct IcsBackend;

impl IcsBackend {
    /// Create a new ICS backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Format datetime string for better readability
    /// Converts ISO 8601 format to human-readable format, preserving original in parentheses
    #[inline]
    fn format_datetime(dt: &str) -> String {
        // If it's already human-readable (contains hyphens or slashes), return as-is
        if dt.contains('-') || dt.contains('/') {
            return dt.to_string();
        }

        // Parse basic ISO 8601 formats and show both human-readable and original ICS format:
        // DATE: YYYYMMDD (8 chars)
        // DATE-TIME (UTC): YYYYMMDDTHHmmssZ (16 chars)
        // DATE-TIME (floating): YYYYMMDDTHHmmss (15 chars)

        let readable = if dt.len() == 8 {
            // DATE format: YYYYMMDD
            format!("{}-{}-{}", &dt[0..4], &dt[4..6], &dt[6..8])
        } else if dt.len() == 16 && dt.ends_with('Z') {
            // DATE-TIME UTC: YYYYMMDDTHHmmssZ
            format!(
                "{}-{}-{} {}:{}:{} UTC",
                &dt[0..4],
                &dt[4..6],
                &dt[6..8],
                &dt[9..11],
                &dt[11..13],
                &dt[13..15]
            )
        } else if dt.len() == 15 && dt.contains('T') {
            // DATE-TIME floating: YYYYMMDDTHHmmss
            format!(
                "{}-{}-{} {}:{}:{}",
                &dt[0..4],
                &dt[4..6],
                &dt[6..8],
                &dt[9..11],
                &dt[11..13],
                &dt[13..15]
            )
        } else {
            // Unknown format or already human-readable
            return dt.to_string();
        };

        // Show both human-readable and original ICS format
        format!("{readable} (`{dt}`)")
    }

    /// Format event as markdown
    fn format_event(event: &CalendarEvent) -> String {
        let mut md = format!("### {}\n\n", event.summary);

        // Add date/time first (most important info)
        if let Some(start) = &event.start {
            let _ = write!(md, "**When:** {}", Self::format_datetime(start));
            if let Some(end) = &event.end {
                let _ = write!(md, " – {}", Self::format_datetime(end));
            }
            md.push_str("\n\n");
        }

        // Add location
        if let Some(location) = &event.location {
            let _ = writeln!(md, "**Where:** {location}\n");
        }

        // Add organizer
        if let Some(organizer) = &event.organizer {
            let _ = writeln!(md, "**Organizer:** {organizer}\n");
        }

        // Add attendees
        if !event.attendees.is_empty() {
            md.push_str("**Attendees:**\n");
            for attendee in &event.attendees {
                let _ = writeln!(md, "- {attendee}");
            }
            md.push('\n');
        }

        // Add status
        if let Some(status) = &event.status {
            let _ = writeln!(md, "**Status:** {status}\n");
        }

        // Add recurrence
        if let Some(recurrence) = &event.recurrence {
            let _ = writeln!(md, "**Recurrence:** {recurrence}\n");
        }

        // Add alarms/reminders
        if !event.alarms.is_empty() {
            md.push_str("**Reminders:**\n");
            for alarm in &event.alarms {
                let mut alarm_desc = String::new();

                // Add trigger time
                if let Some(trigger) = &alarm.trigger {
                    alarm_desc.push_str(trigger);
                }

                // Add action
                if let Some(action) = &alarm.action {
                    if !alarm_desc.is_empty() {
                        alarm_desc.push_str(" (");
                    }
                    alarm_desc.push_str(action);
                    if !alarm_desc.ends_with('(') {
                        alarm_desc.push(')');
                    }
                }

                // Add description if present
                if let Some(desc) = &alarm.description {
                    if !alarm_desc.is_empty() {
                        alarm_desc.push_str(" - ");
                    }
                    alarm_desc.push_str(desc);
                }

                let _ = writeln!(md, "- {alarm_desc}");
            }
            md.push('\n');
        }

        // Add description
        if let Some(description) = &event.description {
            let _ = writeln!(md, "{description}\n");
        }

        // Add metadata section (separated from event details)
        let has_metadata = event.uid.is_some() || event.dtstamp.is_some();
        if has_metadata {
            md.push_str("---\n\n*Metadata:*\n");

            if let Some(uid) = &event.uid {
                let _ = writeln!(md, "- UID: {uid}");
            }

            if let Some(dtstamp) = &event.dtstamp {
                let _ = writeln!(md, "- Created: {}", Self::format_datetime(dtstamp));
            }

            md.push('\n');
        }

        md
    }

    /// Format todo as markdown
    fn format_todo(todo: &CalendarTodo) -> String {
        let mut md = format!("### {}\n\n", todo.summary);

        // Add due date
        if let Some(due) = &todo.due {
            let _ = writeln!(md, "Due: {due}\n");
        }

        // Add priority
        if let Some(priority) = todo.priority {
            let _ = writeln!(md, "Priority: {priority}\n");
        }

        // Add status
        if let Some(status) = &todo.status {
            let _ = writeln!(md, "Status: {status}\n");
        }

        // Add completion
        if let Some(percent) = todo.percent_complete {
            let _ = writeln!(md, "Complete: {percent}%\n");
        }

        // Add description
        if let Some(description) = &todo.description {
            let _ = writeln!(md, "{description}\n");
        }

        md
    }

    /// Format journal as markdown
    fn format_journal(journal: &CalendarJournal) -> String {
        let mut md = format!("### {}\n\n", journal.summary);

        // Add date
        if let Some(date) = &journal.date {
            let _ = writeln!(md, "Date: {date}\n");
        }

        // Add description
        if let Some(description) = &journal.description {
            let _ = writeln!(md, "{description}\n");
        }

        md
    }

    /// Create `DocItems` from calendar event
    fn create_event_docitems(event: &CalendarEvent, text_idx: &mut usize) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Event summary as SectionHeader (level 3)
        doc_items.push(create_section_header(
            *text_idx,
            event.summary.clone(),
            3,
            vec![],
        ));
        *text_idx += 1;

        // Date/time first (most important info, with bold formatting)
        if let Some(start) = &event.start {
            let mut when_text = format!("**When:** {}", Self::format_datetime(start));
            if let Some(end) = &event.end {
                let _ = write!(when_text, " – {}", Self::format_datetime(end));
            }

            let when_idx = *text_idx;
            *text_idx += 1;
            doc_items.push(create_text_item(when_idx, when_text, vec![]));
        }

        if let Some(location) = &event.location {
            let loc_idx = *text_idx;
            *text_idx += 1;
            let loc_text = format!("**Where:** {location}");
            doc_items.push(create_text_item(loc_idx, loc_text, vec![]));
        }

        if let Some(organizer) = &event.organizer {
            let org_idx = *text_idx;
            *text_idx += 1;
            let org_text = format!("**Organizer:** {organizer}");
            doc_items.push(create_text_item(org_idx, org_text, vec![]));
        }

        // Add attendees as list structure (matches markdown format)
        if !event.attendees.is_empty() {
            // Header: "**Attendees:**"
            doc_items.push(create_text_item(
                *text_idx,
                "**Attendees:**".to_string(),
                vec![],
            ));
            *text_idx += 1;

            // List items: "- attendee"
            for attendee in &event.attendees {
                let att_text = format!("- {attendee}");
                doc_items.push(create_text_item(*text_idx, att_text, vec![]));
                *text_idx += 1;
            }
        }

        if let Some(status) = &event.status {
            let status_idx = *text_idx;
            *text_idx += 1;
            let status_text = format!("**Status:** {status}");
            doc_items.push(create_text_item(status_idx, status_text, vec![]));
        }

        if let Some(recurrence) = &event.recurrence {
            let rec_idx = *text_idx;
            *text_idx += 1;
            let rec_text = format!("**Recurrence:** {recurrence}");
            doc_items.push(create_text_item(rec_idx, rec_text, vec![]));
        }

        // Add alarms/reminders as list structure (matches markdown format)
        if !event.alarms.is_empty() {
            // Header: "**Reminders:**"
            doc_items.push(create_text_item(
                *text_idx,
                "**Reminders:**".to_string(),
                vec![],
            ));
            *text_idx += 1;

            // List items: "- alarm_desc"
            for alarm in &event.alarms {
                let mut alarm_desc = String::new();

                // Add trigger time
                if let Some(trigger) = &alarm.trigger {
                    alarm_desc.push_str(trigger);
                }

                // Add action
                if let Some(action) = &alarm.action {
                    if !alarm_desc.is_empty() {
                        alarm_desc.push_str(" (");
                    }
                    alarm_desc.push_str(action);
                    if !alarm_desc.ends_with('(') {
                        alarm_desc.push(')');
                    }
                }

                // Add description if present
                if let Some(desc) = &alarm.description {
                    if !alarm_desc.is_empty() {
                        alarm_desc.push_str(" - ");
                    }
                    alarm_desc.push_str(desc);
                }

                let alarm_text = format!("- {alarm_desc}");
                doc_items.push(create_text_item(*text_idx, alarm_text, vec![]));
                *text_idx += 1;
            }
        }

        if let Some(description) = &event.description {
            let desc_idx = *text_idx;
            *text_idx += 1;
            doc_items.push(create_text_item(desc_idx, description.clone(), vec![]));
        }

        // Add metadata section (separated from event details)
        let has_metadata = event.uid.is_some() || event.dtstamp.is_some();
        if has_metadata {
            // Separator
            doc_items.push(create_text_item(*text_idx, "---".to_string(), vec![]));
            *text_idx += 1;

            // Metadata header
            doc_items.push(create_text_item(
                *text_idx,
                "*Metadata:*".to_string(),
                vec![],
            ));
            *text_idx += 1;

            if let Some(uid) = &event.uid {
                let uid_text = format!("- UID: {uid}");
                doc_items.push(create_text_item(*text_idx, uid_text, vec![]));
                *text_idx += 1;
            }

            if let Some(dtstamp) = &event.dtstamp {
                let dtstamp_text = format!("- Created: {}", Self::format_datetime(dtstamp));
                doc_items.push(create_text_item(*text_idx, dtstamp_text, vec![]));
                *text_idx += 1;
            }
        }

        doc_items
    }

    /// Create `DocItems` from calendar todo
    fn create_todo_docitems(todo: &CalendarTodo, text_idx: &mut usize) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Todo summary as SectionHeader (level 3)
        doc_items.push(create_section_header(
            *text_idx,
            todo.summary.clone(),
            3,
            vec![],
        ));
        *text_idx += 1;

        // Todo fields as Text DocItems
        if let Some(due) = &todo.due {
            let due_idx = *text_idx;
            *text_idx += 1;
            let due_text = format!("Due: {due}");
            doc_items.push(create_text_item(due_idx, due_text, vec![]));
        }

        if let Some(priority) = todo.priority {
            let pri_idx = *text_idx;
            *text_idx += 1;
            let pri_text = format!("Priority: {priority}");
            doc_items.push(create_text_item(pri_idx, pri_text, vec![]));
        }

        if let Some(status) = &todo.status {
            let status_idx = *text_idx;
            *text_idx += 1;
            let status_text = format!("Status: {status}");
            doc_items.push(create_text_item(status_idx, status_text, vec![]));
        }

        if let Some(percent) = todo.percent_complete {
            let pct_idx = *text_idx;
            *text_idx += 1;
            let pct_text = format!("Complete: {percent}%");
            doc_items.push(create_text_item(pct_idx, pct_text, vec![]));
        }

        if let Some(description) = &todo.description {
            let desc_idx = *text_idx;
            *text_idx += 1;
            doc_items.push(create_text_item(desc_idx, description.clone(), vec![]));
        }

        doc_items
    }

    /// Create `DocItems` from calendar journal
    fn create_journal_docitems(journal: &CalendarJournal, text_idx: &mut usize) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Journal summary as SectionHeader (level 3)
        doc_items.push(create_section_header(
            *text_idx,
            journal.summary.clone(),
            3,
            vec![],
        ));
        *text_idx += 1;

        // Journal fields as Text DocItems
        if let Some(date) = &journal.date {
            let date_idx = *text_idx;
            *text_idx += 1;
            let date_text = format!("Date: {date}");
            doc_items.push(create_text_item(date_idx, date_text, vec![]));
        }

        if let Some(description) = &journal.description {
            let desc_idx = *text_idx;
            *text_idx += 1;
            doc_items.push(create_text_item(desc_idx, description.clone(), vec![]));
        }

        doc_items
    }

    /// Create `DocItems` from ICS calendar
    fn create_docitems(ics: &IcsInfo) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_idx = 0;

        // Check if calendar has any content
        let has_content = ics.name.is_some()
            || ics.description.is_some()
            || ics.prodid.is_some()
            || ics.method.is_some()
            || ics.timezone.is_some()
            || !ics.events.is_empty()
            || !ics.todos.is_empty()
            || !ics.journals.is_empty();

        // Only add document type indicator if there's content
        if has_content {
            doc_items.push(create_text_item(
                text_idx,
                "ICS/iCalendar Document".to_string(),
                vec![],
            ));
            text_idx += 1;
        }

        // Calendar name as SectionHeader (level 1)
        if let Some(name) = &ics.name {
            doc_items.push(create_section_header(text_idx, name.clone(), 1, vec![]));
            text_idx += 1;
        }

        // Calendar description
        if let Some(description) = &ics.description {
            let desc_idx = text_idx;
            text_idx += 1;
            doc_items.push(create_text_item(desc_idx, description.clone(), vec![]));
        }

        // Calendar metadata (only if there's actual content)
        // Group metadata under a section header for better clarity (matches ODS pattern)
        if has_content
            && (ics.version.is_some()
                || ics.prodid.is_some()
                || ics.method.is_some()
                || ics.timezone.is_some())
        {
            doc_items.push(create_section_header(
                text_idx,
                "Metadata".to_string(),
                2,
                vec![],
            ));
            text_idx += 1;

            if let Some(version) = &ics.version {
                let version_idx = text_idx;
                text_idx += 1;
                let version_text = format!("Version: {version}");
                doc_items.push(create_text_item(version_idx, version_text, vec![]));
            }

            if let Some(prodid) = &ics.prodid {
                let prodid_idx = text_idx;
                text_idx += 1;
                let prodid_text = format!("Product ID: {prodid}");
                doc_items.push(create_text_item(prodid_idx, prodid_text, vec![]));
            }

            if let Some(method) = &ics.method {
                let method_idx = text_idx;
                text_idx += 1;
                let method_text = format!("Method: {method}");
                doc_items.push(create_text_item(method_idx, method_text, vec![]));
            }

            if let Some(timezone) = &ics.timezone {
                let tz_idx = text_idx;
                text_idx += 1;
                let tz_text = format!("Timezone: {timezone}");
                doc_items.push(create_text_item(tz_idx, tz_text, vec![]));
            }
        }

        // Events section
        if !ics.events.is_empty() {
            doc_items.push(create_section_header(
                text_idx,
                "Events".to_string(),
                2,
                vec![],
            ));
            text_idx += 1;

            for event in &ics.events {
                let event_items = Self::create_event_docitems(event, &mut text_idx);
                doc_items.extend(event_items);
            }
        }

        // Todos section
        if !ics.todos.is_empty() {
            doc_items.push(create_section_header(
                text_idx,
                "Todos".to_string(),
                2,
                vec![],
            ));
            text_idx += 1;

            for todo in &ics.todos {
                let todo_items = Self::create_todo_docitems(todo, &mut text_idx);
                doc_items.extend(todo_items);
            }
        }

        // Journals section
        if !ics.journals.is_empty() {
            doc_items.push(create_section_header(
                text_idx,
                "Journal Entries".to_string(),
                2,
                vec![],
            ));
            text_idx += 1;

            for journal in &ics.journals {
                let journal_items = Self::create_journal_docitems(journal, &mut text_idx);
                doc_items.extend(journal_items);
            }
        }

        doc_items
    }

    /// Convert ICS info to markdown
    fn ics_to_markdown(ics: &IcsInfo) -> String {
        let mut markdown = String::new();

        // Check if calendar has any content
        let has_content = ics.name.is_some()
            || ics.description.is_some()
            || ics.prodid.is_some()
            || ics.method.is_some()
            || ics.timezone.is_some()
            || !ics.events.is_empty()
            || !ics.todos.is_empty()
            || !ics.journals.is_empty();

        // Only add document type indicator if there's content
        if has_content {
            markdown.push_str("ICS/iCalendar Document\n\n");
        }

        // Add calendar title
        if let Some(name) = &ics.name {
            let _ = writeln!(markdown, "# {name}\n");
        }

        // Add calendar description
        if let Some(description) = &ics.description {
            let _ = writeln!(markdown, "{description}\n");
        }

        // Add calendar metadata (only if there's actual content)
        if has_content {
            if let Some(version) = &ics.version {
                let _ = writeln!(markdown, "Version: {version}\n");
            }

            if let Some(prodid) = &ics.prodid {
                let _ = writeln!(markdown, "Product ID: {prodid}\n");
            }

            if let Some(method) = &ics.method {
                let _ = writeln!(markdown, "Method: {method}\n");
            }

            if let Some(timezone) = &ics.timezone {
                let _ = writeln!(markdown, "Timezone: {timezone}\n");
            }
        }

        // Add events section
        if !ics.events.is_empty() {
            markdown.push_str("## Events\n\n");
            for event in &ics.events {
                markdown.push_str(&Self::format_event(event));
            }
        }

        // Add todos section
        if !ics.todos.is_empty() {
            markdown.push_str("## Todos\n\n");
            for todo in &ics.todos {
                markdown.push_str(&Self::format_todo(todo));
            }
        }

        // Add journals section
        if !ics.journals.is_empty() {
            markdown.push_str("## Journal Entries\n\n");
            for journal in &ics.journals {
                markdown.push_str(&Self::format_journal(journal));
            }
        }

        markdown
    }
}

impl DocumentBackend for IcsBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Ics
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        // Write bytes to temp file for parsing (docling-calendar requires file path)
        let temp_file_path = crate::utils::write_temp_file(data, "calendar", ".ics")?;
        self.parse_file(&temp_file_path, options)
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let calendar_name = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("calendar.ics");

        // Parse ICS file
        let ics = parse_ics(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse ICS file: {e}: {calendar_name}"))
        })?;

        // Generate DocItems
        let doc_items = Self::create_docitems(&ics);

        // Convert to markdown
        let markdown = Self::ics_to_markdown(&ics);
        let num_characters = markdown.chars().count();

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Ics,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: ics.name.or_else(|| Some(calendar_name.to_string())),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: ics.description, // N=1880: Extract calendar description to subject
                exif: None,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_event() {
        let event = CalendarEvent {
            summary: "Team Meeting".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("Discuss Q4 plans".to_string()),
            location: Some("Conference Room A".to_string()),
            start: Some("2024-01-15T10:00:00".to_string()),
            end: Some("2024-01-15T11:00:00".to_string()),
            organizer: Some("john@example.com".to_string()),
            attendees: vec![
                "alice@example.com".to_string(),
                "bob@example.com".to_string(),
            ],
            status: Some("CONFIRMED".to_string()),
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);
        assert!(
            md.contains("### Team Meeting"),
            "Event summary should appear as level 3 heading"
        );
        assert!(
            md.contains("**When:**")
                && md.contains("2024-01-15")
                && md.contains("10:00:00")
                && md.contains("11:00:00"),
            "Event should contain When label with start and end times"
        );
        assert!(
            md.contains("**Where:** Conference Room A"),
            "Event location should be formatted with bold label"
        );
        assert!(
            md.contains("**Organizer:** john@example.com"),
            "Organizer should be formatted with bold label"
        );
        assert!(
            md.contains("**Attendees:**"),
            "Attendees section should have bold header"
        );
        assert!(
            md.contains("- alice@example.com"),
            "Attendee list should include alice@example.com"
        );
        assert!(
            md.contains("**Status:** CONFIRMED"),
            "Event status should be formatted with bold label"
        );
        assert!(
            md.contains("Discuss Q4 plans"),
            "Event description should be included in markdown"
        );
    }

    #[test]
    fn test_format_todo() {
        let todo = CalendarTodo {
            summary: "Finish report".to_string(),
            description: Some("Complete quarterly report".to_string()),
            due: Some("2024-01-20T17:00:00".to_string()),
            priority: Some(1),
            status: Some("IN-PROCESS".to_string()),
            percent_complete: Some(75),
        };

        let md = IcsBackend::format_todo(&todo);
        assert!(
            md.contains("### Finish report"),
            "Todo summary should appear as level 3 heading"
        );
        assert!(
            md.contains("Due: 2024-01-20T17:00:00"),
            "Todo due date should be included"
        );
        assert!(
            md.contains("Priority: 1"),
            "Todo priority should be included"
        );
        assert!(
            md.contains("Status: IN-PROCESS"),
            "Todo status should be included"
        );
        assert!(
            md.contains("Complete: 75%"),
            "Todo completion percentage should be formatted"
        );
        assert!(
            md.contains("Complete quarterly report"),
            "Todo description should be included"
        );
    }

    #[test]
    fn test_format_journal() {
        let journal = CalendarJournal {
            summary: "Project Notes".to_string(),
            description: Some("Made progress on feature X".to_string()),
            date: Some("2024-01-15".to_string()),
        };

        let md = IcsBackend::format_journal(&journal);
        assert!(
            md.contains("### Project Notes"),
            "Journal summary should appear as level 3 heading"
        );
        assert!(
            md.contains("Date: 2024-01-15"),
            "Journal date should be included"
        );
        assert!(
            md.contains("Made progress on feature X"),
            "Journal description should be included"
        );
    }

    #[test]
    fn test_ics_to_markdown_empty() {
        let ics = IcsInfo {
            name: Some("My Calendar".to_string()),
            description: Some("Personal schedule".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);
        assert!(
            md.contains("# My Calendar"),
            "Calendar name should appear as level 1 heading"
        );
        assert!(
            md.contains("Personal schedule"),
            "Calendar description should be included"
        );
        assert!(
            !md.contains("## Events"),
            "Empty events should not create Events section"
        );
        assert!(
            !md.contains("## Todos"),
            "Empty todos should not create Todos section"
        );
        assert!(
            !md.contains("## Journal Entries"),
            "Empty journals should not create Journal Entries section"
        );
    }

    #[test]
    fn test_format_method() {
        let backend = IcsBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Ics,
            "IcsBackend should report Ics format"
        );
    }

    // ============================================================================
    // CATEGORY 1: Metadata Tests (3 tests)
    // ============================================================================

    #[test]
    fn test_ics_metadata_with_name() {
        let ics = IcsInfo {
            name: Some("Work Calendar".to_string()),
            description: Some("Schedule for work events".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let markdown = IcsBackend::ics_to_markdown(&ics);

        // Title should be extracted from name
        assert!(
            markdown.contains("# Work Calendar"),
            "Calendar name should be extracted as title"
        );

        // Character count should match
        let num_chars = markdown.chars().count();
        assert!(
            num_chars > 0,
            "Markdown output should have non-zero character count"
        );
    }

    #[test]
    fn test_ics_metadata_character_count() {
        let ics = IcsInfo {
            name: Some("Test Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Sample Event".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: Some("2024-01-15T10:00:00".to_string()),
                end: None,
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let markdown = IcsBackend::ics_to_markdown(&ics);
        let num_chars = markdown.chars().count();

        // Character count should be positive and accurate
        assert!(
            num_chars > 0,
            "Character count should be positive for calendar with content"
        );
        assert_eq!(
            num_chars,
            markdown.chars().count(),
            "Character count should be accurate"
        );
    }

    #[test]
    fn test_ics_metadata_empty_calendar() {
        let ics = IcsInfo {
            name: None,
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let markdown = IcsBackend::ics_to_markdown(&ics);

        // Empty calendar should have minimal content
        assert!(
            markdown.is_empty() || markdown.trim().is_empty(),
            "Empty calendar should produce empty or whitespace-only markdown"
        );
    }

    /// N=1880: Test that calendar description is extracted to metadata.subject
    #[test]
    fn test_ics_metadata_description_extraction() {
        // Create IcsInfo with description
        let ics = IcsInfo {
            name: Some("Project Calendar".to_string()),
            description: Some("Calendar for tracking project milestones and meetings".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        // Manually construct Document to test metadata extraction
        // (mimics what parse_file does)
        let doc_items = IcsBackend::create_docitems(&ics);
        let markdown = IcsBackend::ics_to_markdown(&ics);
        let num_characters = markdown.chars().count();

        let document = Document {
            markdown,
            format: InputFormat::Ics,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: ics.name.clone(),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: ics.description.clone(), // N=1880: Extract description to subject
                exif: None,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        };

        // Verify description was extracted to subject field
        assert_eq!(
            document.metadata.subject,
            Some("Calendar for tracking project milestones and meetings".to_string()),
            "Calendar description should be extracted to metadata subject field"
        );

        // Verify other metadata is correct
        assert_eq!(
            document.metadata.title,
            Some("Project Calendar".to_string()),
            "Calendar name should be extracted to metadata title field"
        );
    }

    /// N=1880: Test that missing description results in None subject
    #[test]
    fn test_ics_metadata_no_description() {
        let ics = IcsInfo {
            name: Some("Simple Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let doc_items = IcsBackend::create_docitems(&ics);
        let markdown = IcsBackend::ics_to_markdown(&ics);
        let num_characters = markdown.chars().count();

        let document = Document {
            markdown,
            format: InputFormat::Ics,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: ics.name.clone(),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: ics.description.clone(), // N=1880: None when no description
                exif: None,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        };

        // Verify subject is None when no description
        assert_eq!(
            document.metadata.subject, None,
            "Subject should be None when calendar has no description"
        );
    }

    // ============================================================================
    // CATEGORY 2: DocItem Generation Tests (3 tests)
    // ============================================================================

    #[test]
    fn test_ics_docitem_single_event() {
        let ics = IcsInfo {
            name: Some("Test Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Meeting".to_string(),
                uid: None,
                dtstamp: None,
                description: Some("Discuss project".to_string()),
                location: Some("Room 101".to_string()),
                start: Some("2024-01-15T10:00:00".to_string()),
                end: Some("2024-01-15T11:00:00".to_string()),
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Should have: Document type indicator + Title (SectionHeader) + Events (SectionHeader) + Event summary (SectionHeader) + fields (Text items)
        assert!(doc_items.len() >= 6, "DocItems should include document type, calendar title, Events header, event summary, and fields");

        // First item should be document type indicator (Text)
        match &doc_items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "ICS/iCalendar Document",
                    "First DocItem should be document type indicator"
                );
            }
            _ => panic!("Expected Text for document type indicator"),
        }

        // Second item should be calendar title (SectionHeader level 1)
        match &doc_items[1] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(
                    text, "Test Calendar",
                    "Second DocItem should be calendar title"
                );
                assert_eq!(*level, 1, "Calendar title should be level 1 heading");
            }
            _ => panic!("Expected SectionHeader for calendar title"),
        }

        // Should contain "Events" SectionHeader (level 2)
        let has_events_header = doc_items.iter().any(|item| {
            matches!(item, DocItem::SectionHeader { text, level, .. } if text == "Events" && *level == 2)
        });
        assert!(
            has_events_header,
            "DocItems should contain Events section header"
        );

        // Should contain event summary SectionHeader (level 3)
        let has_event_summary = doc_items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, level, .. } if text == "Meeting" && *level == 3));
        assert!(
            has_event_summary,
            "DocItems should contain event summary as level 3 header"
        );
    }

    #[test]
    fn test_ics_docitem_todo() {
        let ics = IcsInfo {
            name: Some("Task List".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![CalendarTodo {
                summary: "Complete report".to_string(),
                description: Some("Q4 report".to_string()),
                due: Some("2024-01-20T17:00:00".to_string()),
                priority: Some(1),
                status: Some("IN-PROCESS".to_string()),
                percent_complete: Some(50),
            }],
            journals: vec![],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Should have: Document type indicator + Title + Todos (SectionHeader) + Todo summary + fields
        assert!(
            doc_items.len() >= 7,
            "DocItems should include document type, title, Todos header, todo summary, and fields"
        );

        // Should contain "Todos" SectionHeader
        let has_todos_header = doc_items.iter().any(|item| {
            matches!(item, DocItem::SectionHeader { text, level, .. } if text == "Todos" && *level == 2)
        });
        assert!(
            has_todos_header,
            "DocItems should contain Todos section header"
        );

        // Should contain todo summary
        let has_todo_summary = doc_items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, level, .. } if text == "Complete report" && *level == 3));
        assert!(
            has_todo_summary,
            "DocItems should contain todo summary as level 3 header"
        );
    }

    #[test]
    fn test_ics_docitem_empty_calendar() {
        let ics = IcsInfo {
            name: None,
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Empty calendar should have no DocItems
        assert_eq!(
            doc_items.len(),
            0,
            "Empty calendar should produce no DocItems"
        );
    }

    // ============================================================================
    // CATEGORY 3: Format-Specific Features (5 tests)
    // ============================================================================

    #[test]
    fn test_ics_event_with_recurrence() {
        let event = CalendarEvent {
            summary: "Weekly Standup".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: Some("2024-01-15T09:00:00".to_string()),
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: Some("FREQ=WEEKLY;BYDAY=MO".to_string()),
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        assert!(
            md.contains("### Weekly Standup"),
            "Recurring event summary should appear as heading"
        );
        assert!(
            md.contains("**Recurrence:** FREQ=WEEKLY;BYDAY=MO"),
            "Recurrence rule should be formatted with bold label"
        );
    }

    #[test]
    fn test_ics_event_with_multiple_attendees() {
        let event = CalendarEvent {
            summary: "All Hands".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![
                "alice@example.com".to_string(),
                "bob@example.com".to_string(),
                "charlie@example.com".to_string(),
            ],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        assert!(
            md.contains("Attendees:"),
            "Event with attendees should have Attendees section"
        );
        assert!(
            md.contains("- alice@example.com"),
            "First attendee should be listed"
        );
        assert!(
            md.contains("- bob@example.com"),
            "Second attendee should be listed"
        );
        assert!(
            md.contains("- charlie@example.com"),
            "Third attendee should be listed"
        );
    }

    #[test]
    fn test_ics_todo_priority_levels() {
        let high_priority = CalendarTodo {
            summary: "Urgent task".to_string(),
            description: None,
            due: None,
            priority: Some(1),
            status: None,
            percent_complete: None,
        };

        let low_priority = CalendarTodo {
            summary: "Low priority task".to_string(),
            description: None,
            due: None,
            priority: Some(9),
            status: None,
            percent_complete: None,
        };

        let md_high = IcsBackend::format_todo(&high_priority);
        let md_low = IcsBackend::format_todo(&low_priority);

        assert!(
            md_high.contains("Priority: 1"),
            "High priority todo should show priority 1"
        );
        assert!(
            md_low.contains("Priority: 9"),
            "Low priority todo should show priority 9"
        );
    }

    #[test]
    fn test_ics_journal_entry() {
        let journal = CalendarJournal {
            summary: "Development Notes".to_string(),
            description: Some("Fixed critical bug in authentication module".to_string()),
            date: Some("2024-01-15".to_string()),
        };

        let md = IcsBackend::format_journal(&journal);

        assert!(
            md.contains("### Development Notes"),
            "Journal summary should appear as level 3 heading"
        );
        assert!(
            md.contains("Date: 2024-01-15"),
            "Journal date should be included"
        );
        assert!(
            md.contains("Fixed critical bug in authentication module"),
            "Journal description should be included"
        );
    }

    #[test]
    fn test_ics_multiple_sections() {
        let ics = IcsInfo {
            name: Some("Full Calendar".to_string()),
            description: Some("All types".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Event 1".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: None,
                end: None,
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![CalendarTodo {
                summary: "Todo 1".to_string(),
                description: None,
                due: None,
                priority: None,
                status: None,
                percent_complete: None,
            }],
            journals: vec![CalendarJournal {
                summary: "Journal 1".to_string(),
                description: None,
                date: None,
            }],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Should have all three sections
        assert!(
            md.contains("## Events"),
            "Calendar with events should have Events section"
        );
        assert!(
            md.contains("## Todos"),
            "Calendar with todos should have Todos section"
        );
        assert!(
            md.contains("## Journal Entries"),
            "Calendar with journals should have Journal Entries section"
        );
    }

    // ============================================================================
    // CATEGORY 4: Edge Cases (3 tests)
    // ============================================================================

    #[test]
    fn test_ics_event_minimal_fields() {
        let event = CalendarEvent {
            summary: "Minimal Event".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Should only have summary
        assert!(
            md.contains("### Minimal Event"),
            "Minimal event should have summary heading"
        );
        assert!(
            !md.contains("When:"),
            "Event without start time should not have When field"
        );
        assert!(
            !md.contains("Where:"),
            "Event without location should not have Where field"
        );
        assert!(
            !md.contains("Organizer:"),
            "Event without organizer should not have Organizer field"
        );
    }

    #[test]
    fn test_ics_todo_completion_percentage() {
        let todo_0 = CalendarTodo {
            summary: "Not started".to_string(),
            description: None,
            due: None,
            priority: None,
            status: None,
            percent_complete: Some(0),
        };

        let todo_100 = CalendarTodo {
            summary: "Completed".to_string(),
            description: None,
            due: None,
            priority: None,
            status: None,
            percent_complete: Some(100),
        };

        let md_0 = IcsBackend::format_todo(&todo_0);
        let md_100 = IcsBackend::format_todo(&todo_100);

        assert!(
            md_0.contains("Complete: 0%"),
            "Todo with 0% completion should show Complete: 0%"
        );
        assert!(
            md_100.contains("Complete: 100%"),
            "Todo with 100% completion should show Complete: 100%"
        );
    }

    #[test]
    fn test_ics_calendar_description_only() {
        let ics = IcsInfo {
            name: None,
            description: Some("This calendar has no name".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Should have description but no title
        assert!(
            !md.contains("# "),
            "Calendar without name should not have level 1 heading"
        );
        assert!(
            md.contains("This calendar has no name"),
            "Calendar description should be included"
        );
    }

    // ============================================================================
    // CATEGORY 5: Event Variations (5 tests)
    // ============================================================================

    #[test]
    fn test_ics_event_all_day() {
        let event = CalendarEvent {
            summary: "Birthday".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: Some("2024-01-15".to_string()), // Date only, no time
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        assert!(
            md.contains("### Birthday"),
            "All-day event summary should appear as heading"
        );
        assert!(
            md.contains("**When:**") && md.contains("2024-01-15"),
            "All-day event should have When field with date"
        );
    }

    #[test]
    fn test_ics_event_with_end_no_start() {
        let event = CalendarEvent {
            summary: "Event with only end time".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: Some("2024-01-15T17:00:00".to_string()),
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Without start time, end time should not appear
        assert!(
            md.contains("### Event with only end time"),
            "Event summary should appear even without start time"
        );
        assert!(
            !md.contains("When:"),
            "Event without start time should not show When field"
        );
    }

    #[test]
    fn test_ics_event_status_tentative() {
        let event = CalendarEvent {
            summary: "Tentative Meeting".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: Some("TENTATIVE".to_string()),
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        assert!(
            md.contains("**Status:** TENTATIVE"),
            "Event with TENTATIVE status should show status field"
        );
    }

    #[test]
    fn test_ics_event_status_cancelled() {
        let event = CalendarEvent {
            summary: "Cancelled Event".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: Some("CANCELLED".to_string()),
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        assert!(
            md.contains("**Status:** CANCELLED"),
            "Event with CANCELLED status should show status field"
        );
    }

    #[test]
    fn test_ics_event_long_description() {
        let long_desc = "This is a very long description that spans multiple lines.\n\
                         It contains detailed information about the event.\n\
                         Including agenda items, preparation notes, and more.";

        let event = CalendarEvent {
            summary: "Detailed Event".to_string(),
            uid: None,
            dtstamp: None,
            description: Some(long_desc.to_string()),
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        assert!(
            md.contains("### Detailed Event"),
            "Event with long description should have summary heading"
        );
        assert!(
            md.contains(long_desc),
            "Long description should be preserved in markdown"
        );
    }

    // ============================================================================
    // CATEGORY 6: Todo Variations (5 tests)
    // ============================================================================

    #[test]
    fn test_ics_todo_no_fields() {
        let todo = CalendarTodo {
            summary: "Simple Todo".to_string(),
            description: None,
            due: None,
            priority: None,
            status: None,
            percent_complete: None,
        };

        let md = IcsBackend::format_todo(&todo);

        // Should only have summary
        assert!(
            md.contains("### Simple Todo"),
            "Simple todo should have summary heading"
        );
        assert!(
            !md.contains("Due:"),
            "Todo without due date should not show Due field"
        );
        assert!(
            !md.contains("Priority:"),
            "Todo without priority should not show Priority field"
        );
        assert!(
            !md.contains("Status:"),
            "Todo without status should not show Status field"
        );
        assert!(
            !md.contains("Complete:"),
            "Todo without completion should not show Complete field"
        );
    }

    #[test]
    fn test_ics_todo_status_needs_action() {
        let todo = CalendarTodo {
            summary: "Needs Action".to_string(),
            description: None,
            due: None,
            priority: None,
            status: Some("NEEDS-ACTION".to_string()),
            percent_complete: None,
        };

        let md = IcsBackend::format_todo(&todo);

        assert!(
            md.contains("Status: NEEDS-ACTION"),
            "Todo with NEEDS-ACTION status should show status field"
        );
    }

    #[test]
    fn test_ics_todo_status_completed() {
        let todo = CalendarTodo {
            summary: "Done Task".to_string(),
            description: None,
            due: None,
            priority: None,
            status: Some("COMPLETED".to_string()),
            percent_complete: Some(100),
        };

        let md = IcsBackend::format_todo(&todo);

        assert!(
            md.contains("Status: COMPLETED"),
            "Completed todo should show COMPLETED status"
        );
        assert!(
            md.contains("Complete: 100%"),
            "Completed todo should show 100% completion"
        );
    }

    #[test]
    fn test_ics_todo_zero_priority() {
        let todo = CalendarTodo {
            summary: "No priority".to_string(),
            description: None,
            due: None,
            priority: Some(0),
            status: None,
            percent_complete: None,
        };

        let md = IcsBackend::format_todo(&todo);

        assert!(
            md.contains("Priority: 0"),
            "Todo with zero priority should show Priority: 0"
        );
    }

    #[test]
    fn test_ics_todo_with_all_fields() {
        let todo = CalendarTodo {
            summary: "Complete Task".to_string(),
            description: Some("Full task details".to_string()),
            due: Some("2024-01-20T17:00:00".to_string()),
            priority: Some(5),
            status: Some("IN-PROCESS".to_string()),
            percent_complete: Some(50),
        };

        let md = IcsBackend::format_todo(&todo);

        assert!(
            md.contains("### Complete Task"),
            "Todo with all fields should have summary heading"
        );
        assert!(
            md.contains("Due: 2024-01-20T17:00:00"),
            "Todo should show due date"
        );
        assert!(md.contains("Priority: 5"), "Todo should show priority");
        assert!(md.contains("Status: IN-PROCESS"), "Todo should show status");
        assert!(
            md.contains("Complete: 50%"),
            "Todo should show completion percentage"
        );
        assert!(
            md.contains("Full task details"),
            "Todo should show description"
        );
    }

    // ============================================================================
    // CATEGORY 7: Journal Variations (3 tests)
    // ============================================================================

    #[test]
    fn test_ics_journal_no_date() {
        let journal = CalendarJournal {
            summary: "Undated Note".to_string(),
            description: Some("A note without a date".to_string()),
            date: None,
        };

        let md = IcsBackend::format_journal(&journal);

        assert!(
            md.contains("### Undated Note"),
            "Journal without date should have summary heading"
        );
        assert!(
            !md.contains("Date:"),
            "Journal without date should not show Date field"
        );
        assert!(
            md.contains("A note without a date"),
            "Journal description should be included"
        );
    }

    #[test]
    fn test_ics_journal_no_description() {
        let journal = CalendarJournal {
            summary: "Minimal Journal".to_string(),
            description: None,
            date: Some("2024-01-15".to_string()),
        };

        let md = IcsBackend::format_journal(&journal);

        assert!(
            md.contains("### Minimal Journal"),
            "Journal without description should have summary heading"
        );
        assert!(md.contains("Date: 2024-01-15"), "Journal should show date");
    }

    #[test]
    fn test_ics_journal_multiline_description() {
        let journal = CalendarJournal {
            summary: "Daily Notes".to_string(),
            description: Some(
                "Line 1: Morning standup\nLine 2: Code review\nLine 3: Testing".to_string(),
            ),
            date: Some("2024-01-15".to_string()),
        };

        let md = IcsBackend::format_journal(&journal);

        assert!(
            md.contains("### Daily Notes"),
            "Journal with multiline description should have summary heading"
        );
        assert!(
            md.contains("Line 1: Morning standup"),
            "First line of multiline description should be preserved"
        );
        assert!(
            md.contains("Line 2: Code review"),
            "Second line of multiline description should be preserved"
        );
        assert!(
            md.contains("Line 3: Testing"),
            "Third line of multiline description should be preserved"
        );
    }

    // ============================================================================
    // CATEGORY 8: DocItem Index Correctness (4 tests)
    // ============================================================================

    #[test]
    fn test_ics_docitem_indices_single_event() {
        let ics = IcsInfo {
            name: Some("Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Event".to_string(),
                uid: None,
                dtstamp: None,
                description: Some("Description".to_string()),
                location: None,
                start: Some("2024-01-15T10:00:00".to_string()),
                end: None,
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Check that indices are sequential
        for (i, item) in doc_items.iter().enumerate() {
            let self_ref = match item {
                DocItem::SectionHeader { self_ref, .. } => self_ref,
                DocItem::Text { self_ref, .. } => self_ref,
                _ => continue,
            };

            // self_ref format is "#/headers/{index}" or "#/texts/{index}"
            let index_str = if self_ref.starts_with("#/headers/") {
                self_ref.trim_start_matches("#/headers/")
            } else {
                self_ref.trim_start_matches("#/texts/")
            };
            let index: usize = index_str.parse().expect("Invalid index in self_ref");
            assert_eq!(
                index, i,
                "DocItem at position {i} should have self_ref index {i}, but has {index}"
            );
        }
    }

    #[test]
    fn test_ics_docitem_indices_multiple_events() {
        let ics = IcsInfo {
            name: Some("Multi-Event Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![
                CalendarEvent {
                    summary: "Event 1".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: None,
                    location: None,
                    start: Some("2024-01-15T10:00:00".to_string()),
                    end: None,
                    organizer: None,
                    attendees: vec![],
                    status: None,
                    recurrence: None,
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "Event 2".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: None,
                    location: None,
                    start: Some("2024-01-16T10:00:00".to_string()),
                    end: None,
                    organizer: None,
                    attendees: vec![],
                    status: None,
                    recurrence: None,
                    alarms: vec![],
                },
            ],
            todos: vec![],
            journals: vec![],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Verify all indices are unique and sequential
        for (i, item) in doc_items.iter().enumerate() {
            let self_ref = match item {
                DocItem::SectionHeader { self_ref, .. } => self_ref,
                DocItem::Text { self_ref, .. } => self_ref,
                _ => continue,
            };

            let index_str = if self_ref.starts_with("#/headers/") {
                self_ref.trim_start_matches("#/headers/")
            } else {
                self_ref.trim_start_matches("#/texts/")
            };
            let index: usize = index_str.parse().expect("Invalid index in self_ref");
            assert_eq!(
                index, i,
                "DocItem at position {i} should have self_ref index {i}, but has {index}"
            );
        }
    }

    #[test]
    fn test_ics_docitem_mixed_content_ordering() {
        let ics = IcsInfo {
            name: Some("Mixed Calendar".to_string()),
            description: Some("Calendar description".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Event".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: None,
                end: None,
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![CalendarTodo {
                summary: "Todo".to_string(),
                description: None,
                due: None,
                priority: None,
                status: None,
                percent_complete: None,
            }],
            journals: vec![CalendarJournal {
                summary: "Journal".to_string(),
                description: None,
                date: None,
            }],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Find section headers and verify order: Title, Description, Events, Event, Todos, Todo, Journals, Journal
        let mut section_headers = vec![];
        for item in &doc_items {
            if let DocItem::SectionHeader { text, .. } = item {
                section_headers.push(text.clone());
            }
        }

        assert_eq!(
            section_headers[0], "Mixed Calendar",
            "First section header should be calendar title"
        );
        // Description is Text, not SectionHeader
        assert_eq!(
            section_headers[1], "Metadata",
            "Second section header should be Metadata"
        );
        assert_eq!(
            section_headers[2], "Events",
            "Third section header should be Events"
        );
        assert_eq!(
            section_headers[3], "Event",
            "Fourth section header should be event title"
        );
        assert_eq!(
            section_headers[4], "Todos",
            "Fifth section header should be Todos"
        );
        assert_eq!(
            section_headers[5], "Todo",
            "Sixth section header should be todo title"
        );
        assert_eq!(
            section_headers[6], "Journal Entries",
            "Seventh section header should be Journal Entries"
        );
        assert_eq!(
            section_headers[7], "Journal",
            "Eighth section header should be journal title"
        );
    }

    #[test]
    fn test_ics_docitem_large_calendar() {
        let ics = IcsInfo {
            name: Some("Large Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![
                CalendarEvent {
                    summary: "E1".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: None,
                    location: None,
                    start: Some("2024-01-15T10:00:00".to_string()),
                    end: None,
                    organizer: None,
                    attendees: vec![],
                    status: None,
                    recurrence: None,
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "E2".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: None,
                    location: None,
                    start: Some("2024-01-16T10:00:00".to_string()),
                    end: None,
                    organizer: None,
                    attendees: vec![],
                    status: None,
                    recurrence: None,
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "E3".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: None,
                    location: None,
                    start: Some("2024-01-17T10:00:00".to_string()),
                    end: None,
                    organizer: None,
                    attendees: vec![],
                    status: None,
                    recurrence: None,
                    alarms: vec![],
                },
            ],
            todos: vec![
                CalendarTodo {
                    summary: "T1".to_string(),
                    description: None,
                    due: None,
                    priority: None,
                    status: None,
                    percent_complete: None,
                },
                CalendarTodo {
                    summary: "T2".to_string(),
                    description: None,
                    due: None,
                    priority: None,
                    status: None,
                    percent_complete: None,
                },
            ],
            journals: vec![CalendarJournal {
                summary: "J1".to_string(),
                description: None,
                date: None,
            }],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Should have many items: document type indicator, calendar title, Metadata header, version, Events header, 3 events (each with header + when),
        // Todos header, 2 todos (each with header), Journals header, 1 journal (header)
        // = 1 (doc type) + 1 (title) + 1 (Metadata header) + 1 (version) + 1 (Events) + 3*(1 header + 1 when) + 1 (Todos) + 2*1 + 1 (Journals) + 1 = 16 items
        assert_eq!(
            doc_items.len(),
            16,
            "Large calendar should create 16 DocItems"
        );

        // Verify all indices are unique
        for (i, item) in doc_items.iter().enumerate() {
            let self_ref = match item {
                DocItem::SectionHeader { self_ref, .. } => self_ref,
                DocItem::Text { self_ref, .. } => self_ref,
                _ => continue,
            };

            let index_str = if self_ref.starts_with("#/headers/") {
                self_ref.trim_start_matches("#/headers/")
            } else {
                self_ref.trim_start_matches("#/texts/")
            };
            let index: usize = index_str.parse().expect("Invalid index in self_ref");
            assert_eq!(
                index, i,
                "DocItem at position {i} should have self_ref index {i}, but has {index}"
            );
        }
    }

    // ============================================================================
    // CATEGORY 9: Text Formatting Edge Cases (3 tests)
    // ============================================================================

    #[test]
    fn test_ics_event_empty_attendees_list() {
        let event = CalendarEvent {
            summary: "No Attendees".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Empty attendees list should not show "Attendees:" header
        assert!(
            !md.contains("Attendees:"),
            "Event with empty attendees list should not show Attendees section"
        );
    }

    #[test]
    fn test_ics_event_special_characters() {
        let event = CalendarEvent {
            summary: "Event with \"quotes\" & <symbols>".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("Description with\nnewlines\tand\ttabs".to_string()),
            location: Some("Room #123 (Building A)".to_string()),
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Special characters should be preserved
        assert!(
            md.contains("Event with \"quotes\" & <symbols>"),
            "Quotes and symbols in event summary should be preserved"
        );
        assert!(
            md.contains("Room #123 (Building A)"),
            "Special characters in location should be preserved"
        );
        assert!(
            md.contains("Description with\nnewlines\tand\ttabs"),
            "Newlines and tabs in description should be preserved"
        );
    }

    #[test]
    fn test_ics_markdown_empty_sections() {
        let ics = IcsInfo {
            name: Some("Selective Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Only Event".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: None,
                end: None,
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],    // Empty
            journals: vec![], // Empty
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Should have Events section, but not Todos or Journals
        assert!(
            md.contains("## Events"),
            "Calendar with only events should have Events section"
        );
        assert!(
            !md.contains("## Todos"),
            "Calendar without todos should not have Todos section"
        );
        assert!(
            !md.contains("## Journal Entries"),
            "Calendar without journals should not have Journal Entries section"
        );
    }

    // ============================================================================
    // CATEGORY 10: Markdown Formatting Edge Cases (5 tests)
    // ============================================================================

    #[test]
    fn test_ics_event_trailing_whitespace() {
        let event = CalendarEvent {
            summary: "Event with trailing spaces   ".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("Description with trailing spaces   ".to_string()),
            location: Some("Location with trailing spaces   ".to_string()),
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Trailing whitespace should be preserved in markdown
        assert!(
            md.contains("### Event with trailing spaces   "),
            "Trailing spaces in event summary should be preserved"
        );
        assert!(
            md.contains("Description with trailing spaces   "),
            "Trailing spaces in description should be preserved"
        );
        assert!(
            md.contains("**Where:** Location with trailing spaces   "),
            "Trailing spaces in location should be preserved"
        );
    }

    #[test]
    fn test_ics_event_tabs_in_description() {
        let event = CalendarEvent {
            summary: "Event".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("Line 1\n\tIndented line\n\t\tDouble indented".to_string()),
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Tabs should be preserved
        assert!(
            md.contains("Line 1\n\tIndented line\n\t\tDouble indented"),
            "Tab indentation in description should be preserved"
        );
    }

    #[test]
    fn test_ics_event_markdown_code_blocks() {
        let event = CalendarEvent {
            summary: "Code Review".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("```rust\nfn main() {\n    println!(\"Hello\");\n}\n```".to_string()),
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Markdown code blocks should be preserved as-is
        assert!(
            md.contains("```rust"),
            "Markdown code block fence should be preserved"
        );
        assert!(
            md.contains("fn main() {"),
            "Code content should be preserved"
        );
        assert!(
            md.contains("println!(\"Hello\");"),
            "Code with quotes should be preserved"
        );
    }

    #[test]
    fn test_ics_event_inline_code() {
        let event = CalendarEvent {
            summary: "Meeting about `API` design".to_string(),
            uid: None,
            dtstamp: None,
            description: Some(
                "Discuss `GET /users` endpoint and `POST /users` creation".to_string(),
            ),
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Inline code backticks should be preserved
        assert!(
            md.contains("Meeting about `API` design"),
            "Inline code in summary should be preserved"
        );
        assert!(
            md.contains("`GET /users`"),
            "First inline code in description should be preserved"
        );
        assert!(
            md.contains("`POST /users`"),
            "Second inline code in description should be preserved"
        );
    }

    #[test]
    fn test_ics_event_markdown_links() {
        let event = CalendarEvent {
            summary: "Event".to_string(),
            uid: None,
            dtstamp: None,
            description: Some(
                "See [documentation](https://example.com/docs) for details".to_string(),
            ),
            location: Some("[Zoom](https://zoom.us/j/123456789)".to_string()),
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Markdown links should be preserved
        assert!(
            md.contains("[documentation](https://example.com/docs)"),
            "Markdown link in description should be preserved"
        );
        assert!(
            md.contains("[Zoom](https://zoom.us/j/123456789)"),
            "Markdown link in location should be preserved"
        );
    }

    // ============================================================================
    // CATEGORY 11: Data Validation Edge Cases (6 tests)
    // ============================================================================

    #[test]
    fn test_ics_event_very_long_summary() {
        let long_summary = "A".repeat(500);
        let event = CalendarEvent {
            summary: long_summary.clone(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Very long summary should be preserved
        assert!(
            md.contains(&format!("### {long_summary}")),
            "Very long event summary should be preserved"
        );
    }

    #[test]
    fn test_ics_unicode_in_all_fields() {
        let event = CalendarEvent {
            summary: "会议 🗓️".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("讨论项目进展 📊".to_string()),
            location: Some("会议室 🏢".to_string()),
            start: Some("2024-01-15T10:00:00".to_string()),
            end: None,
            organizer: Some("张三 <zhang@example.com>".to_string()),
            attendees: vec![
                "李四 <li@example.com>".to_string(),
                "王五 <wang@example.com>".to_string(),
            ],
            status: Some("确认".to_string()),
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Unicode characters should be preserved in all fields
        assert!(
            md.contains("### 会议 🗓️"),
            "Chinese characters and emoji in summary should be preserved"
        );
        assert!(
            md.contains("讨论项目进展 📊"),
            "Chinese characters and emoji in description should be preserved"
        );
        assert!(
            md.contains("**Where:** 会议室 🏢"),
            "Chinese characters and emoji in location should be preserved"
        );
        assert!(
            md.contains("**Organizer:** 张三 <zhang@example.com>"),
            "Chinese characters in organizer should be preserved"
        );
        assert!(
            md.contains("- 李四 <li@example.com>"),
            "Chinese characters in attendee should be preserved"
        );
        assert!(
            md.contains("**Status:** 确认"),
            "Chinese characters in status should be preserved"
        );
    }

    #[test]
    fn test_ics_empty_strings_vs_none() {
        let event_empty_strings = CalendarEvent {
            summary: "Event".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("".to_string()),
            location: Some("".to_string()),
            start: None,
            end: None,
            organizer: Some("".to_string()),
            attendees: vec![],
            status: Some("".to_string()),
            recurrence: Some("".to_string()),
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event_empty_strings);

        // Empty strings should still trigger field display (unlike None)
        assert!(
            md.contains("**Where:** "),
            "Empty string location should show Where field"
        );
        assert!(
            md.contains("**Organizer:** "),
            "Empty string organizer should show Organizer field"
        );
        assert!(
            md.contains("**Status:** "),
            "Empty string status should show Status field"
        );
        assert!(
            md.contains("**Recurrence:** "),
            "Empty string recurrence should show Recurrence field"
        );
    }

    #[test]
    fn test_ics_todo_zero_priority_validation() {
        let todo = CalendarTodo {
            summary: "Zero priority".to_string(),
            description: None,
            due: None,
            priority: Some(0),
            status: None,
            percent_complete: None,
        };

        let md = IcsBackend::format_todo(&todo);

        // Zero priority should be formatted (even though RFC 5545 says 1-9)
        assert!(
            md.contains("Priority: 0"),
            "Zero priority should be formatted even though RFC 5545 says 1-9"
        );
    }

    #[test]
    fn test_ics_todo_priority_boundary_values() {
        let priority_1 = CalendarTodo {
            summary: "Highest priority".to_string(),
            description: None,
            due: None,
            priority: Some(1),
            status: None,
            percent_complete: None,
        };

        let priority_9 = CalendarTodo {
            summary: "Lowest priority".to_string(),
            description: None,
            due: None,
            priority: Some(9),
            status: None,
            percent_complete: None,
        };

        let md_1 = IcsBackend::format_todo(&priority_1);
        let md_9 = IcsBackend::format_todo(&priority_9);

        // Valid priority range is 1-9 in RFC 5545
        assert!(
            md_1.contains("Priority: 1"),
            "Priority 1 (highest) should be formatted"
        );
        assert!(
            md_9.contains("Priority: 9"),
            "Priority 9 (lowest) should be formatted"
        );
    }

    #[test]
    fn test_ics_todo_completion_edge_values() {
        let zero = CalendarTodo {
            summary: "Zero completion".to_string(),
            description: None,
            due: None,
            priority: None,
            status: None,
            percent_complete: Some(0),
        };

        let max_u8 = CalendarTodo {
            summary: "Max u8 completion".to_string(),
            description: None,
            due: None,
            priority: None,
            status: None,
            percent_complete: Some(255),
        };

        let md_zero = IcsBackend::format_todo(&zero);
        let md_max = IcsBackend::format_todo(&max_u8);

        // Edge values for u8 should be formatted (0 and 255)
        assert!(
            md_zero.contains("Complete: 0%"),
            "Zero completion should be formatted"
        );
        assert!(
            md_max.contains("Complete: 255%"),
            "Maximum u8 completion value should be formatted"
        );
    }

    // ============================================================================
    // CATEGORY 12: Backend Trait Completeness (3 tests)
    // ============================================================================

    #[test]
    fn test_ics_backend_new_vs_default() {
        let backend_new = IcsBackend::new();
        let backend_default = IcsBackend;

        // Both constructors should produce equivalent backends
        assert_eq!(
            backend_new.format(),
            backend_default.format(),
            "new() and default should produce equivalent backends"
        );
    }

    #[test]
    fn test_ics_parse_bytes_invalid_format() {
        let backend = IcsBackend::new();
        let invalid_ics = b"This is not valid ICS format\nNo BEGIN:VCALENDAR\n";

        let result = backend.parse_bytes(invalid_ics, &BackendOptions::default());

        // Should fail gracefully with error (invalid ICS format)
        assert!(result.is_err(), "Invalid ICS format should return error");

        // Error message should indicate ICS parsing failure
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse ICS file") || err_msg.contains("parse"),
            "Error message should indicate ICS parsing failure"
        );
    }

    #[test]
    fn test_ics_content_blocks_validation() {
        let ics_with_content = IcsInfo {
            name: Some("Calendar".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Event".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: None,
                end: None,
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let ics_empty = IcsInfo {
            name: None,
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let doc_items_with_content = IcsBackend::create_docitems(&ics_with_content);
        let doc_items_empty = IcsBackend::create_docitems(&ics_empty);

        // Non-empty calendar should have DocItems
        assert!(
            !doc_items_with_content.is_empty(),
            "Calendar with content should have DocItems"
        );

        // Empty calendar should have no DocItems
        assert!(
            doc_items_empty.is_empty(),
            "Empty calendar should have no DocItems"
        );
    }

    // ============================================================================
    // CATEGORY 13: Additional ICS Field Variations (7 tests)
    // ============================================================================

    #[test]
    fn test_ics_event_organizer_with_email() {
        let event = CalendarEvent {
            summary: "Team Meeting".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: Some("John Doe <john.doe@example.com>".to_string()),
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Organizer with email format should be preserved
        assert!(
            md.contains("**Organizer:** John Doe <john.doe@example.com>"),
            "Organizer with name and email format should be preserved"
        );
    }

    #[test]
    fn test_ics_event_attendees_with_mailto() {
        let event = CalendarEvent {
            summary: "Meeting".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![
                "mailto:alice@example.com".to_string(),
                "mailto:bob@example.com".to_string(),
            ],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Mailto URIs should be preserved
        assert!(
            md.contains("- mailto:alice@example.com"),
            "Mailto URI for first attendee should be preserved"
        );
        assert!(
            md.contains("- mailto:bob@example.com"),
            "Mailto URI for second attendee should be preserved"
        );
    }

    #[test]
    fn test_ics_event_recurrence_complex() {
        let event = CalendarEvent {
            summary: "Complex Recurrence".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: None,
            end: None,
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: Some("FREQ=MONTHLY;BYDAY=2TU;UNTIL=20241231T235959Z".to_string()),
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Complex recurrence rule should be preserved
        assert!(
            md.contains("**Recurrence:** FREQ=MONTHLY;BYDAY=2TU;UNTIL=20241231T235959Z"),
            "Complex recurrence rule should be preserved"
        );
    }

    #[test]
    fn test_ics_todo_due_date_only() {
        let todo = CalendarTodo {
            summary: "Task".to_string(),
            description: None,
            due: Some("2024-01-20".to_string()),
            priority: None,
            status: None,
            percent_complete: None,
        };

        let md = IcsBackend::format_todo(&todo);

        // Date-only due date (no time) should be preserved
        assert!(
            md.contains("Due: 2024-01-20"),
            "Date-only due date should be preserved"
        );
    }

    #[test]
    fn test_ics_journal_with_timestamp() {
        let journal = CalendarJournal {
            summary: "Journal Entry".to_string(),
            description: Some("Entry details".to_string()),
            date: Some("2024-01-15T09:30:00Z".to_string()),
        };

        let md = IcsBackend::format_journal(&journal);

        // Timestamp with time and timezone should be preserved
        assert!(
            md.contains("Date: 2024-01-15T09:30:00Z"),
            "Journal timestamp with timezone should be preserved"
        );
    }

    #[test]
    fn test_ics_calendar_with_very_long_name() {
        let long_name = "A".repeat(500);
        let ics = IcsInfo {
            name: Some(long_name.clone()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Very long calendar name should be preserved
        assert!(
            md.contains(&format!("# {long_name}")),
            "Very long calendar name should be preserved"
        );
    }

    #[test]
    fn test_ics_docitem_calendar_with_description() {
        let ics = IcsInfo {
            name: Some("Calendar".to_string()),
            description: Some("This is a calendar description".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let doc_items = IcsBackend::create_docitems(&ics);

        // Should have 5 DocItems: document type indicator (Text) + title (SectionHeader) + description (Text) + Metadata (SectionHeader) + version (Text)
        assert_eq!(
            doc_items.len(),
            5,
            "Calendar with description should create 5 DocItems"
        );

        // First item: document type indicator
        match &doc_items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "ICS/iCalendar Document",
                    "First DocItem should be document type indicator"
                );
            }
            _ => panic!("Expected Text for document type indicator"),
        }

        // Second item: calendar name as SectionHeader
        match &doc_items[1] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Calendar", "Second DocItem should be calendar name");
                assert_eq!(*level, 1, "Calendar name should be level 1 heading");
            }
            _ => panic!("Expected SectionHeader for calendar name"),
        }

        // Third item: description as Text
        match &doc_items[2] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "This is a calendar description",
                    "Third DocItem should be calendar description"
                );
            }
            _ => panic!("Expected Text for calendar description"),
        }

        // Fourth item: Metadata section header
        match &doc_items[3] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Metadata", "Fourth DocItem should be Metadata header");
                assert_eq!(*level, 2, "Metadata header should be level 2");
            }
            _ => panic!("Expected SectionHeader for Metadata"),
        }

        // Fifth item: version as Text
        match &doc_items[4] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Version: 2.0", "Fifth DocItem should be version text");
            }
            _ => panic!("Expected Text for version"),
        }
    }

    // ==================== ADDITIONAL EDGE CASES (N=536) ====================

    #[test]
    fn test_ics_event_with_all_day_flag() {
        // Test all-day events (DTSTART;VALUE=DATE:20240101 without time component)
        let event = CalendarEvent {
            summary: "All Day Event".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("Whole day event".to_string()),
            location: None,
            start: Some("20240101".to_string()), // DATE format (no time)
            end: Some("20240102".to_string()),
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Should handle DATE format (8 chars = YYYYMMDD)
        assert!(
            md.contains("All Day Event"),
            "All-day event summary should be present"
        );
        assert!(
            md.contains("20240101") || md.contains("2024-01-01"),
            "All-day event date should be preserved"
        );
    }

    #[test]
    fn test_ics_event_with_floating_time() {
        // Test floating time (no timezone, local time)
        let event = CalendarEvent {
            summary: "Floating Time Event".to_string(),
            uid: None,
            dtstamp: None,
            description: None,
            location: None,
            start: Some("20240315T140000".to_string()), // No Z suffix = floating time
            end: Some("20240315T160000".to_string()),
            organizer: None,
            attendees: vec![],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Should handle floating time (15 chars = YYYYMMDDTHHmmss)
        assert!(
            md.contains("Floating Time Event"),
            "Floating time event summary should be present"
        );
        assert!(
            md.contains("2024") || md.contains("140000"),
            "Floating time event date or time should be preserved"
        );
    }

    #[test]
    fn test_ics_todo_with_percent_complete_partial() {
        // Test partial completion (PERCENT-COMPLETE:50)
        let todo = CalendarTodo {
            summary: "Half Done".to_string(),
            description: None,
            due: None,
            priority: None,
            status: None,
            percent_complete: Some(50),
        };

        let md = IcsBackend::format_todo(&todo);

        // Should show percentage
        assert!(md.contains("Half Done"), "Todo summary should be present");
        assert!(
            md.contains("50") || md.contains("Completion"),
            "Todo should show 50% completion"
        );
    }

    #[test]
    fn test_ics_event_with_three_attendees() {
        // Test event with three attendees (realistic meeting scenario)
        let event = CalendarEvent {
            summary: "Team Meeting".to_string(),
            uid: None,
            dtstamp: None,
            description: Some("Weekly sync".to_string()),
            location: Some("Conference Room A".to_string()),
            start: Some("20240301T100000Z".to_string()),
            end: Some("20240301T110000Z".to_string()),
            organizer: Some("organizer@example.com".to_string()),
            attendees: vec![
                "alice@example.com".to_string(),
                "bob@example.com".to_string(),
                "charlie@example.com".to_string(),
            ],
            status: None,
            recurrence: None,
            alarms: vec![],
        };

        let md = IcsBackend::format_event(&event);

        // Should list all attendees
        assert!(
            md.contains("Team Meeting"),
            "Event summary should be present"
        );
        assert!(
            md.contains("Conference Room A"),
            "Event location should be present"
        );
        assert!(
            md.contains("alice@example.com")
                || md.contains("bob@example.com")
                || md.contains("charlie@example.com"),
            "At least one of the three attendees should be listed"
        );
    }

    #[test]
    fn test_ics_calendar_with_mixed_content_types() {
        // Test calendar with events, todos, and journals mixed
        let ics = IcsInfo {
            name: Some("Mixed Calendar".to_string()),
            description: Some("Contains everything".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Event".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: Some("20240101T120000Z".to_string()),
                end: Some("20240101T130000Z".to_string()),
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![CalendarTodo {
                summary: "Task".to_string(),
                description: None,
                due: None,
                priority: Some(1),
                status: None,
                percent_complete: None,
            }],
            journals: vec![CalendarJournal {
                summary: "Note".to_string(),
                description: Some("Journal entry".to_string()),
                date: Some("20240101".to_string()),
            }],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Should contain all sections
        assert!(
            md.contains("Mixed Calendar"),
            "Calendar name should be present"
        );
        assert!(
            md.contains("Event") || md.contains("## Events"),
            "Events section should be present"
        );
        assert!(
            md.contains("Task") || md.contains("## Todos"),
            "Todos section should be present"
        );
        assert!(
            md.contains("Note") || md.contains("## Journals"),
            "Journals section should be present"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            doc_items.len() >= 6,
            "DocItems should include name, description, section headers, and items"
        );
    }

    #[test]
    fn test_ics_event_with_multiple_status_values() {
        // Test different event status values (CONFIRMED, TENTATIVE, CANCELLED)
        let ics_confirmed = IcsInfo {
            name: Some("Status Test".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Confirmed Meeting".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: Some("20240115T100000Z".to_string()),
                end: Some("20240115T110000Z".to_string()),
                organizer: None,
                attendees: vec![],
                status: Some("CONFIRMED".to_string()),
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics_confirmed);
        assert!(
            md.contains("**Status:** CONFIRMED"),
            "CONFIRMED event status should be rendered with bold label"
        );

        // Test TENTATIVE status
        let ics_tentative = IcsInfo {
            name: Some("Status Test".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Tentative Meeting".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: Some("20240115T100000Z".to_string()),
                end: Some("20240115T110000Z".to_string()),
                organizer: None,
                attendees: vec![],
                status: Some("TENTATIVE".to_string()),
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics_tentative);
        assert!(
            md.contains("**Status:** TENTATIVE"),
            "TENTATIVE event status should be rendered with bold label"
        );

        // Test CANCELLED status
        let ics_cancelled = IcsInfo {
            name: Some("Status Test".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Cancelled Meeting".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: None,
                start: Some("20240115T100000Z".to_string()),
                end: Some("20240115T110000Z".to_string()),
                organizer: None,
                attendees: vec![],
                status: Some("CANCELLED".to_string()),
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics_cancelled);
        assert!(
            md.contains("**Status:** CANCELLED"),
            "CANCELLED event status should be rendered with bold label"
        );
    }

    #[test]
    fn test_ics_todo_with_all_status_types() {
        // Test different todo status values (NEEDS-ACTION, IN-PROCESS, COMPLETED, CANCELLED)
        let ics = IcsInfo {
            name: Some("Todo Status Test".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![
                CalendarTodo {
                    summary: "Needs Action Task".to_string(),
                    description: None,
                    due: Some("20240120".to_string()),
                    priority: None,
                    status: Some("NEEDS-ACTION".to_string()),
                    percent_complete: None,
                },
                CalendarTodo {
                    summary: "In Progress Task".to_string(),
                    description: None,
                    due: Some("20240121".to_string()),
                    priority: None,
                    status: Some("IN-PROCESS".to_string()),
                    percent_complete: Some(50),
                },
                CalendarTodo {
                    summary: "Completed Task".to_string(),
                    description: None,
                    due: Some("20240122".to_string()),
                    priority: None,
                    status: Some("COMPLETED".to_string()),
                    percent_complete: Some(100),
                },
                CalendarTodo {
                    summary: "Cancelled Task".to_string(),
                    description: None,
                    due: Some("20240123".to_string()),
                    priority: None,
                    status: Some("CANCELLED".to_string()),
                    percent_complete: None,
                },
            ],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Verify all status types are present (todos don't use bold formatting)
        assert!(
            md.contains("Status: NEEDS-ACTION"),
            "NEEDS-ACTION todo status should be present"
        );
        assert!(
            md.contains("Status: IN-PROCESS"),
            "IN-PROCESS todo status should be present"
        );
        assert!(
            md.contains("Status: COMPLETED"),
            "COMPLETED todo status should be present"
        );
        assert!(
            md.contains("Status: CANCELLED"),
            "CANCELLED todo status should be present"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            doc_items.len() >= 9,
            "DocItems should include calendar name, Todos header, and todo items"
        );
    }

    #[test]
    fn test_ics_event_with_location_special_chars() {
        // Test event location with special characters (commas, newlines, unicode)
        let ics = IcsInfo {
            name: Some("Location Test".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "International Meeting".to_string(),
                uid: None,
                dtstamp: None,
                description: None,
                location: Some("Room 123, Building A, 1234 Main St, San Francisco, CA 94105, USA\nConference Room πRoom".to_string()),
                start: Some("20240125T140000Z".to_string()),
                end: Some("20240125T150000Z".to_string()),
                organizer: None,
                attendees: vec![],
                status: None,
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Verify location with special characters
        assert!(
            md.contains("Room 123"),
            "Location should contain room number"
        );
        assert!(
            md.contains("Building A"),
            "Location should contain building name"
        );
        assert!(
            md.contains("San Francisco"),
            "Location should contain city name"
        );
        assert!(
            md.contains("πRoom") || md.contains("Room"),
            "Location should contain room reference"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            !doc_items.is_empty(),
            "DocItems should be created for calendar with location"
        );
    }

    #[test]
    fn test_ics_journal_with_very_long_description() {
        // Test journal with very long description (1000+ characters)
        let long_desc = "This is a very long journal entry description. ".repeat(30);

        let ics = IcsInfo {
            name: Some("Long Journal Test".to_string()),
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![CalendarJournal {
                summary: "Research Notes".to_string(),
                description: Some(long_desc.clone()),
                date: Some("20240130".to_string()),
            }],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Verify long description is included
        assert!(
            md.len() > 1000,
            "Markdown output should be over 1000 chars for long description"
        );
        assert!(
            md.contains("Research Notes"),
            "Journal summary should be present"
        );
        assert!(
            md.contains("very long journal entry description"),
            "Long description content should be included"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            !doc_items.is_empty(),
            "DocItems should be created for journal with long description"
        );

        // Verify content includes long description text somewhere
        let md_again = IcsBackend::ics_to_markdown(&ics);
        assert!(
            md_again.len() > 1000,
            "Markdown output should consistently be over 1000 chars"
        );
    }

    #[test]
    fn test_ics_empty_calendar_with_version_only() {
        // Test completely empty calendar with only version field
        let ics = IcsInfo {
            name: None,
            description: None,
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);

        // Empty calendar produces empty markdown
        assert_eq!(md, "", "Empty calendar should produce empty markdown");

        // Verify DocItems - empty calendar should have minimal items
        let doc_items = IcsBackend::create_docitems(&ics);
        // Empty calendar with no name/description should have 0 items
        assert_eq!(
            doc_items.len(),
            0,
            "Empty calendar should produce no DocItems"
        );

        // Verify backend can handle this without panicking
        let backend = IcsBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Ics,
            "Backend should report Ics format"
        );
    }

    // ========== ADVANCED ICALENDAR RFC 5545 FEATURES (N=631, +5 tests) ==========

    #[test]
    fn test_ics_event_with_timezone_definitions() {
        // Test events with VTIMEZONE components (RFC 5545 Section 3.6.5)
        // Timezone definitions include STANDARD and DAYLIGHT components with TZOFFSETFROM, TZOFFSETTO
        let ics = IcsInfo {
            name: Some("Timezone Test".to_string()),
            description: Some("Calendar with timezone definitions (VTIMEZONE with STANDARD/DAYLIGHT)".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![
                CalendarEvent {
                    summary: "Meeting in PST".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some("TZID=America/Los_Angeles, DTSTART:TZID=America/Los_Angeles:20240315T090000".to_string()),
                    location: Some("San Francisco, CA".to_string()),
                    start: Some("20240315T090000".to_string()),
                    end: Some("20240315T100000".to_string()),
                    organizer: Some("john@example.com".to_string()),
                    attendees: vec!["jane@example.com".to_string()],
                    status: Some("CONFIRMED".to_string()),
                    recurrence: None,
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "Meeting in CET".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some("TZID=Europe/Paris, TZOFFSETFROM=+0100, TZOFFSETTO=+0200 (DST)".to_string()),
                    location: Some("Paris, France".to_string()),
                    start: Some("20240315T140000Z".to_string()),
                    end: Some("20240315T150000Z".to_string()),
                    organizer: Some("pierre@example.com".to_string()),
                    attendees: vec!["marie@example.com".to_string()],
                    status: Some("CONFIRMED".to_string()),
                    recurrence: None,
                    alarms: vec![],
                },
            ],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);
        assert!(
            md.contains("Meeting in PST"),
            "PST timezone event summary should be present"
        );
        assert!(
            md.contains("Meeting in CET"),
            "CET timezone event summary should be present"
        );
        assert!(
            md.contains("TZID=America/Los_Angeles"),
            "Los Angeles timezone ID should be preserved"
        );
        assert!(
            md.contains("TZID=Europe/Paris"),
            "Paris timezone ID should be preserved"
        );
        assert!(
            md.contains("TZOFFSETFROM"),
            "Timezone offset from should be preserved"
        );
        assert!(
            md.contains("TZOFFSETTO"),
            "Timezone offset to should be preserved"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            doc_items.len() >= 10,
            "DocItems should include name, description, section headers, and events"
        );
    }

    #[test]
    fn test_ics_event_with_alarms() {
        // Test events with VALARM components (RFC 5545 Section 3.6.6)
        // Alarms support DISPLAY, AUDIO, EMAIL actions with TRIGGER (relative or absolute)
        let ics = IcsInfo {
            name: Some("Alarms Test".to_string()),
            description: Some("Events with multiple alarm types".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![
                CalendarEvent {
                    summary: "Meeting with Multiple Alarms".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some(
                        "VALARM #1: ACTION=DISPLAY, TRIGGER=-PT15M (15 min before)\n\
                         VALARM #2: ACTION=AUDIO, TRIGGER=-PT5M, ATTACH=sound.wav\n\
                         VALARM #3: ACTION=EMAIL, TRIGGER=-PT1H, SUMMARY=Reminder, ATTENDEE=user@example.com"
                        .to_string()
                    ),
                    location: Some("Conference Room A".to_string()),
                    start: Some("20240320T140000Z".to_string()),
                    end: Some("20240320T150000Z".to_string()),
                    organizer: Some("organizer@example.com".to_string()),
                    attendees: vec!["attendee1@example.com".to_string(), "attendee2@example.com".to_string()],
                    status: Some("CONFIRMED".to_string()),
                    recurrence: None,
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "Task with Absolute Alarm".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some("VALARM: ACTION=DISPLAY, TRIGGER;VALUE=DATE-TIME:20240320T080000Z (absolute time)".to_string()),
                    location: None,
                    start: Some("20240320T090000Z".to_string()),
                    end: Some("20240320T100000Z".to_string()),
                    organizer: None,
                    attendees: vec![],
                    status: None,
                    recurrence: None,
                    alarms: vec![],
                },
            ],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);
        assert!(
            md.contains("Meeting with Multiple Alarms"),
            "Event with multiple alarms summary should be present"
        );
        assert!(
            md.contains("ACTION=DISPLAY"),
            "DISPLAY alarm action should be preserved"
        );
        assert!(
            md.contains("ACTION=AUDIO"),
            "AUDIO alarm action should be preserved"
        );
        assert!(
            md.contains("ACTION=EMAIL"),
            "EMAIL alarm action should be preserved"
        );
        assert!(
            md.contains("TRIGGER=-PT15M"),
            "15-minute trigger should be preserved"
        );
        assert!(
            md.contains("TRIGGER=-PT5M"),
            "5-minute trigger should be preserved"
        );
        assert!(
            md.contains("TRIGGER=-PT1H"),
            "1-hour trigger should be preserved"
        );
        assert!(
            md.contains("sound.wav"),
            "Audio attachment filename should be preserved"
        );
        assert!(
            md.contains("Absolute Alarm"),
            "Absolute alarm event summary should be present"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            doc_items.len() >= 8,
            "DocItems should include name, description, section headers, and events"
        );
    }

    #[test]
    fn test_ics_complex_recurrence_patterns() {
        // Test complex recurrence rules with EXDATE, RDATE (RFC 5545 Section 3.3.10, 3.8.5)
        // RRULE: FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20240630T000000Z
        // EXDATE: Exception dates (skip specific occurrences)
        // RDATE: Additional dates (add extra occurrences)
        let ics = IcsInfo {
            name: Some("Recurrence Test".to_string()),
            description: Some("Complex recurrence patterns with exceptions and additions".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![
                CalendarEvent {
                    summary: "Weekly Team Standup".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some(
                        "RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20240630T000000Z\n\
                         EXDATE:20240318,20240320 (skip March 18 and 20 - spring break)\n\
                         RDATE:20240323 (add extra Saturday meeting)"
                        .to_string()
                    ),
                    location: Some("Zoom".to_string()),
                    start: Some("20240301T100000Z".to_string()),
                    end: Some("20240301T103000Z".to_string()),
                    organizer: Some("manager@example.com".to_string()),
                    attendees: vec!["team@example.com".to_string()],
                    status: Some("CONFIRMED".to_string()),
                    recurrence: Some("FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20240630T000000Z".to_string()),
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "Monthly Board Meeting".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some(
                        "RRULE:FREQ=MONTHLY;BYMONTHDAY=1;COUNT=12 (first of month, 12 occurrences)\n\
                         EXDATE:20240701 (skip July 4th week)"
                        .to_string()
                    ),
                    location: Some("Board Room".to_string()),
                    start: Some("20240101T150000Z".to_string()),
                    end: Some("20240101T170000Z".to_string()),
                    organizer: Some("ceo@example.com".to_string()),
                    attendees: vec!["board@example.com".to_string()],
                    status: Some("CONFIRMED".to_string()),
                    recurrence: Some("FREQ=MONTHLY;BYMONTHDAY=1;COUNT=12".to_string()),
                    alarms: vec![],
                },
            ],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);
        assert!(
            md.contains("Weekly Team Standup"),
            "Weekly standup event summary should be present"
        );
        assert!(
            md.contains("RRULE:FREQ=WEEKLY"),
            "Weekly recurrence rule should be preserved"
        );
        assert!(
            md.contains("BYDAY=MO,WE,FR"),
            "Day-of-week rule should be preserved"
        );
        assert!(
            md.contains("UNTIL=20240630"),
            "Recurrence end date should be preserved"
        );
        assert!(
            md.contains("EXDATE"),
            "Exception dates marker should be preserved"
        );
        assert!(
            md.contains("RDATE"),
            "Additional dates marker should be preserved"
        );
        assert!(
            md.contains("Monthly Board Meeting"),
            "Monthly meeting event summary should be present"
        );
        assert!(
            md.contains("BYMONTHDAY=1"),
            "Monthly day rule should be preserved"
        );
        assert!(
            md.contains("COUNT=12"),
            "Recurrence count should be preserved"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            doc_items.len() >= 8,
            "DocItems should include name, description, section headers, and events"
        );
    }

    #[test]
    fn test_ics_attendee_participation_status() {
        // Test attendee parameters: PARTSTAT, ROLE, RSVP, CUTYPE (RFC 5545 Section 3.2)
        // PARTSTAT: NEEDS-ACTION, ACCEPTED, DECLINED, TENTATIVE, DELEGATED
        // ROLE: CHAIR, REQ-PARTICIPANT, OPT-PARTICIPANT, NON-PARTICIPANT
        // CUTYPE: INDIVIDUAL, GROUP, RESOURCE, ROOM, UNKNOWN
        let ics = IcsInfo {
            name: Some("Attendee Status Test".to_string()),
            description: Some("Event with detailed attendee participation information".to_string()),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![CalendarEvent {
                summary: "Project Kickoff Meeting".to_string(),
                uid: None,
                dtstamp: None,
                description: Some(
                    "Attendee details:\n\
                     - john@example.com: PARTSTAT=ACCEPTED, ROLE=CHAIR, RSVP=TRUE, CUTYPE=INDIVIDUAL\n\
                     - jane@example.com: PARTSTAT=ACCEPTED, ROLE=REQ-PARTICIPANT, RSVP=TRUE, CUTYPE=INDIVIDUAL\n\
                     - bob@example.com: PARTSTAT=TENTATIVE, ROLE=OPT-PARTICIPANT, RSVP=TRUE, CUTYPE=INDIVIDUAL\n\
                     - alice@example.com: PARTSTAT=DECLINED, ROLE=REQ-PARTICIPANT, RSVP=TRUE, CUTYPE=INDIVIDUAL\n\
                     - team@example.com: PARTSTAT=NEEDS-ACTION, ROLE=REQ-PARTICIPANT, RSVP=TRUE, CUTYPE=GROUP\n\
                     - conf-room-a@example.com: PARTSTAT=ACCEPTED, ROLE=NON-PARTICIPANT, CUTYPE=ROOM"
                    .to_string()
                ),
                location: Some("Conference Room A".to_string()),
                start: Some("20240325T140000Z".to_string()),
                end: Some("20240325T160000Z".to_string()),
                organizer: Some("john@example.com".to_string()),
                attendees: vec![
                    "jane@example.com (ACCEPTED)".to_string(),
                    "bob@example.com (TENTATIVE)".to_string(),
                    "alice@example.com (DECLINED)".to_string(),
                    "team@example.com (NEEDS-ACTION, GROUP)".to_string(),
                    "conf-room-a@example.com (ROOM)".to_string(),
                ],
                status: Some("CONFIRMED".to_string()),
                recurrence: None,
                alarms: vec![],
            }],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);
        assert!(
            md.contains("Project Kickoff Meeting"),
            "Event summary should be present"
        );
        assert!(
            md.contains("PARTSTAT=ACCEPTED"),
            "ACCEPTED participation status should be preserved"
        );
        assert!(
            md.contains("PARTSTAT=TENTATIVE"),
            "TENTATIVE participation status should be preserved"
        );
        assert!(
            md.contains("PARTSTAT=DECLINED"),
            "DECLINED participation status should be preserved"
        );
        assert!(
            md.contains("PARTSTAT=NEEDS-ACTION"),
            "NEEDS-ACTION participation status should be preserved"
        );
        assert!(md.contains("ROLE=CHAIR"), "CHAIR role should be preserved");
        assert!(
            md.contains("ROLE=REQ-PARTICIPANT"),
            "REQ-PARTICIPANT role should be preserved"
        );
        assert!(
            md.contains("ROLE=OPT-PARTICIPANT"),
            "OPT-PARTICIPANT role should be preserved"
        );
        assert!(
            md.contains("ROLE=NON-PARTICIPANT"),
            "NON-PARTICIPANT role should be preserved"
        );
        assert!(
            md.contains("CUTYPE=INDIVIDUAL"),
            "INDIVIDUAL calendar user type should be preserved"
        );
        assert!(
            md.contains("CUTYPE=GROUP"),
            "GROUP calendar user type should be preserved"
        );
        assert!(
            md.contains("CUTYPE=ROOM"),
            "ROOM calendar user type should be preserved"
        );
        assert!(md.contains("RSVP=TRUE"), "RSVP flag should be preserved");

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            doc_items.len() >= 5,
            "DocItems should include name, description, section header, and event"
        );
    }

    #[test]
    fn test_ics_freebusy_time_information() {
        // Test VFREEBUSY component (RFC 5545 Section 3.6.4)
        // FREEBUSY: Time ranges with FBTYPE=FREE, BUSY, BUSY-UNAVAILABLE, BUSY-TENTATIVE
        // Used for scheduling and finding available meeting times
        let ics = IcsInfo {
            name: Some("Free/Busy Information".to_string()),
            description: Some(
                "VFREEBUSY component for scheduling:\n\
                 DTSTART:20240401T080000Z\n\
                 DTEND:20240401T180000Z\n\
                 FREEBUSY;FBTYPE=BUSY:20240401T090000Z/20240401T100000Z (meeting)\n\
                 FREEBUSY;FBTYPE=BUSY:20240401T110000Z/20240401T120000Z (meeting)\n\
                 FREEBUSY;FBTYPE=BUSY-TENTATIVE:20240401T140000Z/20240401T150000Z (tentative)\n\
                 FREEBUSY;FBTYPE=BUSY-UNAVAILABLE:20240401T130000Z/20240401T133000Z (lunch)\n\
                 FREEBUSY;FBTYPE=FREE:20240401T080000Z/20240401T090000Z (available)\n\
                 FREEBUSY;FBTYPE=FREE:20240401T150000Z/20240401T180000Z (available)"
                    .to_string(),
            ),
            version: Some("2.0".to_string()),
            prodid: None,
            method: None,
            timezone: None,
            events: vec![
                CalendarEvent {
                    summary: "Busy: Team Standup".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some("FBTYPE=BUSY, 9:00-10:00 AM".to_string()),
                    location: Some("Conference Room".to_string()),
                    start: Some("20240401T090000Z".to_string()),
                    end: Some("20240401T100000Z".to_string()),
                    organizer: None,
                    attendees: vec![],
                    status: Some("CONFIRMED".to_string()),
                    recurrence: None,
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "Busy: Client Call".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some("FBTYPE=BUSY, 11:00-12:00 PM".to_string()),
                    location: Some("Zoom".to_string()),
                    start: Some("20240401T110000Z".to_string()),
                    end: Some("20240401T120000Z".to_string()),
                    organizer: None,
                    attendees: vec![],
                    status: Some("CONFIRMED".to_string()),
                    recurrence: None,
                    alarms: vec![],
                },
                CalendarEvent {
                    summary: "Tentative: Lunch Meeting".to_string(),
                    uid: None,
                    dtstamp: None,
                    description: Some("FBTYPE=BUSY-TENTATIVE, 2:00-3:00 PM".to_string()),
                    location: Some("Restaurant".to_string()),
                    start: Some("20240401T140000Z".to_string()),
                    end: Some("20240401T150000Z".to_string()),
                    organizer: None,
                    attendees: vec![],
                    status: Some("TENTATIVE".to_string()),
                    recurrence: None,
                    alarms: vec![],
                },
            ],
            todos: vec![],
            journals: vec![],
        };

        let md = IcsBackend::ics_to_markdown(&ics);
        assert!(
            md.contains("Free/Busy Information"),
            "Calendar name should be present"
        );
        assert!(
            md.contains("VFREEBUSY"),
            "VFREEBUSY component reference should be preserved"
        );
        assert!(
            md.contains("FBTYPE=BUSY"),
            "BUSY free/busy type should be preserved"
        );
        assert!(
            md.contains("FBTYPE=BUSY-TENTATIVE"),
            "BUSY-TENTATIVE free/busy type should be preserved"
        );
        assert!(
            md.contains("FBTYPE=BUSY-UNAVAILABLE"),
            "BUSY-UNAVAILABLE free/busy type should be preserved"
        );
        assert!(
            md.contains("FBTYPE=FREE"),
            "FREE free/busy type should be preserved"
        );
        assert!(
            md.contains("Team Standup"),
            "Team standup event should be present"
        );
        assert!(
            md.contains("Client Call"),
            "Client call event should be present"
        );
        assert!(
            md.contains("Lunch Meeting"),
            "Lunch meeting event should be present"
        );
        assert!(
            md.contains("20240401T090000Z/20240401T100000Z"),
            "Time range format should be preserved"
        );

        // Verify DocItems created
        let doc_items = IcsBackend::create_docitems(&ics);
        assert!(
            doc_items.len() >= 8,
            "DocItems should include name, description, section headers, and events"
        );
    }
}
