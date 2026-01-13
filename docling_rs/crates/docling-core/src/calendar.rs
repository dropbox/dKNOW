//! Calendar format backend for docling-core
//!
//! Processes ICS/iCalendar files into markdown documents.

use std::fmt::Write;
use std::path::Path;

use crate::error::{DoclingError, Result};

/// Process an ICS/iCalendar file into markdown
///
/// # Arguments
///
/// * `path` - Path to the ICS file
///
/// # Returns
///
/// Returns markdown document with calendar information.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if ICS parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::calendar::process_ics;
///
/// let markdown = process_ics("schedule.ics")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_ics<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    let calendar = docling_calendar::parse_ics(path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse ICS: {e}")))?;

    let mut markdown = String::new();

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("calendar.ics");
    let calendar_name = calendar.name.as_deref().unwrap_or(filename);

    format_calendar_header(&mut markdown, calendar_name, &calendar);

    if !calendar.events.is_empty() {
        format_events_section(&mut markdown, &calendar.events);
    }

    if !calendar.todos.is_empty() {
        format_todos_section(&mut markdown, &calendar.todos);
    }

    if !calendar.journals.is_empty() {
        format_journals_section(&mut markdown, &calendar.journals);
    }

    if calendar.events.is_empty() && calendar.todos.is_empty() && calendar.journals.is_empty() {
        markdown.push_str("*This calendar is empty or contains no parseable entries.*\n\n");
    }

    Ok(markdown)
}

/// Format calendar header and metadata
fn format_calendar_header(md: &mut String, name: &str, calendar: &docling_calendar::IcsInfo) {
    let _ = writeln!(md, "# Calendar: {name}\n");

    if let Some(desc) = &calendar.description {
        let _ = writeln!(md, "{desc}\n");
    }

    md.push_str("## Calendar Information\n\n");
    md.push_str("- **Format:** ICS/iCalendar\n");
    if let Some(version) = &calendar.version {
        let _ = writeln!(md, "- **Version:** {version}");
    }
    let _ = writeln!(md, "- **Events:** {}", calendar.events.len());
    let _ = writeln!(md, "- **Todos:** {}", calendar.todos.len());
    let _ = writeln!(md, "- **Journals:** {}", calendar.journals.len());
    md.push('\n');
}

/// Format events section
#[inline]
fn format_events_section(md: &mut String, events: &[docling_calendar::CalendarEvent]) {
    md.push_str("## Events\n\n");
    for (i, event) in events.iter().enumerate() {
        format_event(md, i + 1, event);
    }
}

/// Format a single calendar event
fn format_event(md: &mut String, num: usize, event: &docling_calendar::CalendarEvent) {
    let _ = writeln!(md, "### {num}. {}\n", event.summary);

    if let Some(desc) = &event.description {
        let _ = writeln!(md, "{desc}\n");
    }

    md.push_str("**Details:**\n\n");
    if let Some(start) = &event.start {
        let _ = writeln!(md, "- **Start:** {start}");
    }
    if let Some(end) = &event.end {
        let _ = writeln!(md, "- **End:** {end}");
    }
    if let Some(location) = &event.location {
        let _ = writeln!(md, "- **Location:** {location}");
    }
    if let Some(organizer) = &event.organizer {
        let _ = writeln!(md, "- **Organizer:** {organizer}");
    }

    if !event.attendees.is_empty() {
        let _ = writeln!(md, "- **Attendees:** {}", event.attendees.len());
        for attendee in &event.attendees {
            let _ = writeln!(md, "  - {attendee}");
        }
    }

    if let Some(status) = &event.status {
        let _ = writeln!(md, "- **Status:** {status}");
    }
    if let Some(recurrence) = &event.recurrence {
        let _ = writeln!(md, "- **Recurrence:** {recurrence}");
    }
    md.push('\n');
}

/// Format todos section
#[inline]
fn format_todos_section(md: &mut String, todos: &[docling_calendar::CalendarTodo]) {
    md.push_str("## To-Do Items\n\n");
    for (i, todo) in todos.iter().enumerate() {
        format_todo(md, i + 1, todo);
    }
}

/// Format a single todo item
fn format_todo(md: &mut String, num: usize, todo: &docling_calendar::CalendarTodo) {
    let status_icon = match todo.status.as_deref() {
        Some("COMPLETED") => "✅",
        Some("IN-PROCESS") => "🔄",
        Some("CANCELLED") => "❌",
        _ => "⬜",
    };

    let _ = writeln!(md, "### {num}. {status_icon} {}\n", todo.summary);

    if let Some(desc) = &todo.description {
        let _ = writeln!(md, "{desc}\n");
    }

    md.push_str("**Details:**\n\n");
    if let Some(due) = &todo.due {
        let _ = writeln!(md, "- **Due:** {due}");
    }
    if let Some(priority) = todo.priority {
        let _ = writeln!(md, "- **Priority:** {priority}");
    }
    if let Some(status) = &todo.status {
        let _ = writeln!(md, "- **Status:** {status}");
    }
    if let Some(percent) = todo.percent_complete {
        let _ = writeln!(md, "- **Complete:** {percent}%");
    }
    md.push('\n');
}

/// Format journals section
#[inline]
fn format_journals_section(md: &mut String, journals: &[docling_calendar::CalendarJournal]) {
    md.push_str("## Journal Entries\n\n");
    for (i, journal) in journals.iter().enumerate() {
        format_journal(md, i + 1, journal);
    }
}

/// Format a single journal entry
fn format_journal(md: &mut String, num: usize, journal: &docling_calendar::CalendarJournal) {
    let _ = writeln!(md, "### {num}. {}\n", journal.summary);

    if let Some(date) = &journal.date {
        let _ = writeln!(md, "**Date:** {date}\n");
    }

    if let Some(desc) = &journal.description {
        let _ = writeln!(md, "{desc}\n");
    }
}
