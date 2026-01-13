//! ICS/iCalendar format parser
//!
//! This module provides parsing for ICS/iCalendar files using the `ical` crate.
//!
//! ## Features
//!
//! - Parse calendar events (VEVENT)
//! - Parse todos (VTODO)
//! - Parse journal entries (VJOURNAL)
//! - Extract event metadata (title, description, location, attendees)
//! - Handle recurring events
//! - Support date/time parsing
//!
//! ## Example
//!
//! ```no_run
//! use docling_calendar::parse_ics;
//!
//! let calendar_info = parse_ics("meeting.ics")?;
//! println!("Found {} events", calendar_info.events.len());
//! # Ok::<(), docling_calendar::CalendarError>(())
//! ```

use crate::error::{CalendarError, Result};
use ical::parser::ical::component::{IcalAlarm, IcalEvent, IcalJournal, IcalTodo};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Information about an ICS calendar file
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct IcsInfo {
    /// Calendar name/title
    pub name: Option<String>,

    /// Calendar description
    pub description: Option<String>,

    /// Calendar version (usually "2.0")
    pub version: Option<String>,

    /// Product identifier (PRODID)
    pub prodid: Option<String>,

    /// Calendar method (METHOD)
    pub method: Option<String>,

    /// Calendar timezone (X-WR-TIMEZONE)
    pub timezone: Option<String>,

    /// List of calendar events
    pub events: Vec<CalendarEvent>,

    /// List of todos
    pub todos: Vec<CalendarTodo>,

    /// List of journal entries
    pub journals: Vec<CalendarJournal>,
}

/// A calendar event (VEVENT)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalendarEvent {
    /// Event summary/title
    pub summary: String,

    /// Event unique identifier (UID)
    pub uid: Option<String>,

    /// Event timestamp (DTSTAMP)
    pub dtstamp: Option<String>,

    /// Event description
    pub description: Option<String>,

    /// Event location
    pub location: Option<String>,

    /// Event start date/time
    pub start: Option<String>,

    /// Event end date/time
    pub end: Option<String>,

    /// Event organizer
    pub organizer: Option<String>,

    /// Event attendees
    pub attendees: Vec<String>,

    /// Event status (e.g., CONFIRMED, TENTATIVE, CANCELLED)
    pub status: Option<String>,

    /// Recurrence rule (RRULE)
    pub recurrence: Option<String>,

    /// Event alarms/reminders (VALARM)
    pub alarms: Vec<CalendarAlarm>,
}

/// A calendar alarm/reminder (VALARM)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalendarAlarm {
    /// Alarm action (e.g., DISPLAY, AUDIO, EMAIL)
    pub action: Option<String>,

    /// Alarm trigger time (e.g., -PT15M for 15 minutes before)
    pub trigger: Option<String>,

    /// Alarm description
    pub description: Option<String>,

    /// Alarm duration (for repeating alarms)
    pub duration: Option<String>,

    /// Alarm repeat count
    pub repeat: Option<u32>,
}

/// A calendar todo (VTODO)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalendarTodo {
    /// Todo summary/title
    pub summary: String,

    /// Todo description
    pub description: Option<String>,

    /// Todo due date
    pub due: Option<String>,

    /// Todo priority (1-9, 1 is highest)
    pub priority: Option<u8>,

    /// Todo status (e.g., NEEDS-ACTION, COMPLETED, IN-PROCESS)
    pub status: Option<String>,

    /// Completion percentage (0-100)
    pub percent_complete: Option<u8>,
}

/// A calendar journal entry (VJOURNAL)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalendarJournal {
    /// Journal summary/title
    pub summary: String,

    /// Journal description/content
    pub description: Option<String>,

    /// Journal date
    pub date: Option<String>,
}

/// Parse ICS file and extract calendar information
///
/// # Arguments
///
/// * `path` - Path to the ICS file
///
/// # Returns
///
/// Returns `IcsInfo` containing the calendar data.
///
/// # Errors
///
/// Returns `CalendarError` if:
/// - File cannot be read
/// - File is not a valid ICS file
/// - Calendar format is unsupported
///
/// # Examples
///
/// ```no_run
/// use docling_calendar::parse_ics;
///
/// let calendar = parse_ics("schedule.ics")?;
/// for event in &calendar.events {
///     println!("Event: {}", event.summary);
/// }
/// # Ok::<(), docling_calendar::CalendarError>(())
/// ```
#[must_use = "this function returns parsed calendar info that should be processed"]
pub fn parse_ics<P: AsRef<Path>>(path: P) -> Result<IcsInfo> {
    let path = path.as_ref();

    // Open ICS file
    let file = File::open(path).map_err(|e| CalendarError::read_error(path, e))?;
    let buf_reader = BufReader::new(file);

    // Parse ICS file
    let reader = ical::IcalParser::new(buf_reader);

    let mut ics_info = IcsInfo {
        name: None,
        description: None,
        version: None,
        prodid: None,
        method: None,
        timezone: None,
        events: Vec::new(),
        todos: Vec::new(),
        journals: Vec::new(),
    };

    // Process each calendar in the file
    for calendar_result in reader {
        let calendar = calendar_result
            .map_err(|e| CalendarError::invalid_format(path, format!("ICS parse error: {e}")))?;

        // Extract calendar properties
        for property in &calendar.properties {
            match property.name.as_str() {
                "X-WR-CALNAME" | "NAME" => {
                    ics_info.name.clone_from(&property.value);
                }
                "X-WR-CALDESC" | "DESCRIPTION" => {
                    ics_info.description.clone_from(&property.value);
                }
                "VERSION" => {
                    ics_info.version.clone_from(&property.value);
                }
                "PRODID" => {
                    ics_info.prodid.clone_from(&property.value);
                }
                "METHOD" => {
                    ics_info.method.clone_from(&property.value);
                }
                "X-WR-TIMEZONE" => {
                    ics_info.timezone.clone_from(&property.value);
                }
                _ => {}
            }
        }

        // Process events
        for event in &calendar.events {
            if let Some(parsed_event) = parse_event(event) {
                ics_info.events.push(parsed_event);
            }
        }

        // Process todos
        for todo in &calendar.todos {
            if let Some(parsed_todo) = parse_todo(todo) {
                ics_info.todos.push(parsed_todo);
            }
        }

        // Process journals
        for journal in &calendar.journals {
            if let Some(parsed_journal) = parse_journal(journal) {
                ics_info.journals.push(parsed_journal);
            }
        }
    }

    Ok(ics_info)
}

/// Parse a VEVENT component
fn parse_event(event: &IcalEvent) -> Option<CalendarEvent> {
    let mut calendar_event = CalendarEvent {
        summary: String::new(),
        uid: None,
        dtstamp: None,
        description: None,
        location: None,
        start: None,
        end: None,
        organizer: None,
        attendees: Vec::new(),
        status: None,
        recurrence: None,
        alarms: Vec::new(),
    };

    for property in &event.properties {
        match property.name.as_str() {
            "SUMMARY" => {
                calendar_event.summary = property.value.clone().unwrap_or_default();
            }
            "UID" => {
                calendar_event.uid.clone_from(&property.value);
            }
            "DTSTAMP" => {
                calendar_event.dtstamp.clone_from(&property.value);
            }
            "DESCRIPTION" => {
                calendar_event.description.clone_from(&property.value);
            }
            "LOCATION" => {
                calendar_event.location.clone_from(&property.value);
            }
            "DTSTART" => {
                calendar_event.start.clone_from(&property.value);
            }
            "DTEND" => {
                calendar_event.end.clone_from(&property.value);
            }
            "ORGANIZER" => {
                calendar_event.organizer.clone_from(&property.value);
            }
            "ATTENDEE" => {
                if let Some(attendee) = property.value.clone() {
                    calendar_event.attendees.push(attendee);
                }
            }
            "STATUS" => {
                calendar_event.status.clone_from(&property.value);
            }
            "RRULE" => {
                calendar_event.recurrence.clone_from(&property.value);
            }
            _ => {}
        }
    }

    // Parse alarms (VALARM components)
    for alarm in &event.alarms {
        if let Some(parsed_alarm) = parse_alarm(alarm) {
            calendar_event.alarms.push(parsed_alarm);
        }
    }

    // Only return events with a summary
    if calendar_event.summary.is_empty() {
        None
    } else {
        Some(calendar_event)
    }
}

/// Parse a VALARM component
fn parse_alarm(alarm: &IcalAlarm) -> Option<CalendarAlarm> {
    let mut calendar_alarm = CalendarAlarm {
        action: None,
        trigger: None,
        description: None,
        duration: None,
        repeat: None,
    };

    for property in &alarm.properties {
        match property.name.as_str() {
            "ACTION" => {
                calendar_alarm.action.clone_from(&property.value);
            }
            "TRIGGER" => {
                calendar_alarm.trigger.clone_from(&property.value);
            }
            "DESCRIPTION" => {
                calendar_alarm.description.clone_from(&property.value);
            }
            "DURATION" => {
                calendar_alarm.duration.clone_from(&property.value);
            }
            "REPEAT" => {
                if let Some(val) = property.value.as_ref() {
                    calendar_alarm.repeat = val.parse().ok();
                }
            }
            _ => {}
        }
    }

    // Return alarm if it has at least a trigger or action
    (calendar_alarm.trigger.is_some() || calendar_alarm.action.is_some()).then_some(calendar_alarm)
}

/// Parse a VTODO component
fn parse_todo(todo: &IcalTodo) -> Option<CalendarTodo> {
    let mut calendar_todo = CalendarTodo {
        summary: String::new(),
        description: None,
        due: None,
        priority: None,
        status: None,
        percent_complete: None,
    };

    for property in &todo.properties {
        match property.name.as_str() {
            "SUMMARY" => {
                calendar_todo.summary = property.value.clone().unwrap_or_default();
            }
            "DESCRIPTION" => {
                calendar_todo.description.clone_from(&property.value);
            }
            "DUE" => {
                calendar_todo.due.clone_from(&property.value);
            }
            "PRIORITY" => {
                if let Some(val) = property.value.as_ref() {
                    calendar_todo.priority = val.parse().ok();
                }
            }
            "STATUS" => {
                calendar_todo.status.clone_from(&property.value);
            }
            "PERCENT-COMPLETE" => {
                if let Some(val) = property.value.as_ref() {
                    calendar_todo.percent_complete = val.parse().ok();
                }
            }
            _ => {}
        }
    }

    // Only return todos with a summary
    if calendar_todo.summary.is_empty() {
        None
    } else {
        Some(calendar_todo)
    }
}

/// Parse a VJOURNAL component
fn parse_journal(journal: &IcalJournal) -> Option<CalendarJournal> {
    let mut calendar_journal = CalendarJournal {
        summary: String::new(),
        description: None,
        date: None,
    };

    for property in &journal.properties {
        match property.name.as_str() {
            "SUMMARY" => {
                calendar_journal.summary = property.value.clone().unwrap_or_default();
            }
            "DESCRIPTION" => {
                calendar_journal.description.clone_from(&property.value);
            }
            "DTSTART" => {
                calendar_journal.date.clone_from(&property.value);
            }
            _ => {}
        }
    }

    // Only return journals with a summary
    if calendar_journal.summary.is_empty() {
        None
    } else {
        Some(calendar_journal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_nonexistent_ics() {
        let result = parse_ics("nonexistent.ics");
        assert!(result.is_err());
    }

    /// Helper to create a temp ICS file with given content
    fn create_temp_ics(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_parse_simple_event() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
SUMMARY:Team Meeting
DTSTART:20251208T100000Z
DTEND:20251208T110000Z
DESCRIPTION:Weekly sync meeting
LOCATION:Conference Room A
UID:test-123@example.com
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.version, Some("2.0".to_string()));
        assert_eq!(result.prodid, Some("-//Test//Test//EN".to_string()));
        assert_eq!(result.events.len(), 1);

        let event = &result.events[0];
        assert_eq!(event.summary, "Team Meeting");
        assert_eq!(event.start, Some("20251208T100000Z".to_string()));
        assert_eq!(event.end, Some("20251208T110000Z".to_string()));
        assert_eq!(event.description, Some("Weekly sync meeting".to_string()));
        assert_eq!(event.location, Some("Conference Room A".to_string()));
        assert_eq!(event.uid, Some("test-123@example.com".to_string()));
        assert_eq!(event.status, Some("CONFIRMED".to_string()));
    }

    #[test]
    fn test_parse_event_with_attendees() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
SUMMARY:Project Review
ORGANIZER:mailto:boss@example.com
ATTENDEE:mailto:alice@example.com
ATTENDEE:mailto:bob@example.com
ATTENDEE:mailto:charlie@example.com
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.events.len(), 1);
        let event = &result.events[0];
        assert_eq!(event.organizer, Some("mailto:boss@example.com".to_string()));
        assert_eq!(event.attendees.len(), 3);
        assert!(event
            .attendees
            .contains(&"mailto:alice@example.com".to_string()));
        assert!(event
            .attendees
            .contains(&"mailto:bob@example.com".to_string()));
        assert!(event
            .attendees
            .contains(&"mailto:charlie@example.com".to_string()));
    }

    #[test]
    fn test_parse_recurring_event() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
SUMMARY:Daily Standup
DTSTART:20251208T090000Z
RRULE:FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.events.len(), 1);
        let event = &result.events[0];
        assert_eq!(event.summary, "Daily Standup");
        assert_eq!(
            event.recurrence,
            Some("FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR".to_string())
        );
    }

    #[test]
    fn test_parse_event_with_alarm() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
SUMMARY:Important Meeting
DTSTART:20251208T140000Z
BEGIN:VALARM
ACTION:DISPLAY
TRIGGER:-PT15M
DESCRIPTION:Meeting starts in 15 minutes
END:VALARM
BEGIN:VALARM
ACTION:AUDIO
TRIGGER:-PT5M
END:VALARM
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.events.len(), 1);
        let event = &result.events[0];
        assert_eq!(event.alarms.len(), 2);

        let alarm1 = &event.alarms[0];
        assert_eq!(alarm1.action, Some("DISPLAY".to_string()));
        assert_eq!(alarm1.trigger, Some("-PT15M".to_string()));
        assert_eq!(
            alarm1.description,
            Some("Meeting starts in 15 minutes".to_string())
        );

        let alarm2 = &event.alarms[1];
        assert_eq!(alarm2.action, Some("AUDIO".to_string()));
        assert_eq!(alarm2.trigger, Some("-PT5M".to_string()));
    }

    #[test]
    fn test_parse_todo() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
SUMMARY:Complete quarterly report
DESCRIPTION:Prepare Q4 financial report
DUE:20251215T170000Z
PRIORITY:1
STATUS:IN-PROCESS
PERCENT-COMPLETE:50
END:VTODO
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.todos.len(), 1);
        let todo = &result.todos[0];
        assert_eq!(todo.summary, "Complete quarterly report");
        assert_eq!(
            todo.description,
            Some("Prepare Q4 financial report".to_string())
        );
        assert_eq!(todo.due, Some("20251215T170000Z".to_string()));
        assert_eq!(todo.priority, Some(1));
        assert_eq!(todo.status, Some("IN-PROCESS".to_string()));
        assert_eq!(todo.percent_complete, Some(50));
    }

    #[test]
    fn test_parse_multiple_todos() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
SUMMARY:Task 1
PRIORITY:3
STATUS:NEEDS-ACTION
END:VTODO
BEGIN:VTODO
SUMMARY:Task 2
PRIORITY:5
STATUS:COMPLETED
PERCENT-COMPLETE:100
END:VTODO
BEGIN:VTODO
SUMMARY:Task 3
PRIORITY:9
END:VTODO
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.todos.len(), 3);
        assert_eq!(result.todos[0].summary, "Task 1");
        assert_eq!(result.todos[0].priority, Some(3));
        assert_eq!(result.todos[1].summary, "Task 2");
        assert_eq!(result.todos[1].percent_complete, Some(100));
        assert_eq!(result.todos[2].summary, "Task 3");
        assert_eq!(result.todos[2].priority, Some(9));
    }

    #[test]
    fn test_parse_journal() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VJOURNAL
SUMMARY:Project retrospective
DESCRIPTION:What went well and areas for improvement
DTSTART:20251208
END:VJOURNAL
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.journals.len(), 1);
        let journal = &result.journals[0];
        assert_eq!(journal.summary, "Project retrospective");
        assert_eq!(
            journal.description,
            Some("What went well and areas for improvement".to_string())
        );
        assert_eq!(journal.date, Some("20251208".to_string()));
    }

    #[test]
    fn test_parse_calendar_with_metadata() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//MyApp//Calendar//EN
METHOD:PUBLISH
X-WR-CALNAME:Work Calendar
X-WR-CALDESC:My work schedule
X-WR-TIMEZONE:America/Los_Angeles
BEGIN:VEVENT
SUMMARY:Test Event
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.name, Some("Work Calendar".to_string()));
        assert_eq!(result.description, Some("My work schedule".to_string()));
        assert_eq!(result.version, Some("2.0".to_string()));
        assert_eq!(result.prodid, Some("-//MyApp//Calendar//EN".to_string()));
        assert_eq!(result.method, Some("PUBLISH".to_string()));
        assert_eq!(result.timezone, Some("America/Los_Angeles".to_string()));
    }

    #[test]
    fn test_parse_mixed_components() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
SUMMARY:Meeting
DTSTART:20251210T100000Z
END:VEVENT
BEGIN:VTODO
SUMMARY:Prepare slides
DUE:20251209T170000Z
END:VTODO
BEGIN:VJOURNAL
SUMMARY:Meeting notes
DESCRIPTION:Action items from today
DTSTART:20251210
END:VJOURNAL
BEGIN:VEVENT
SUMMARY:Follow-up Call
DTSTART:20251211T140000Z
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.events.len(), 2);
        assert_eq!(result.todos.len(), 1);
        assert_eq!(result.journals.len(), 1);

        assert_eq!(result.events[0].summary, "Meeting");
        assert_eq!(result.events[1].summary, "Follow-up Call");
        assert_eq!(result.todos[0].summary, "Prepare slides");
        assert_eq!(result.journals[0].summary, "Meeting notes");
    }

    #[test]
    fn test_skip_event_without_summary() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
DTSTART:20251208T100000Z
DTEND:20251208T110000Z
DESCRIPTION:Event with no summary
END:VEVENT
BEGIN:VEVENT
SUMMARY:Valid Event
DTSTART:20251208T120000Z
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        // Should only have the event with a summary
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].summary, "Valid Event");
    }

    #[test]
    fn test_skip_todo_without_summary() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
DUE:20251215T170000Z
PRIORITY:1
END:VTODO
BEGIN:VTODO
SUMMARY:Valid Todo
END:VTODO
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        // Should only have the todo with a summary
        assert_eq!(result.todos.len(), 1);
        assert_eq!(result.todos[0].summary, "Valid Todo");
    }

    #[test]
    fn test_empty_calendar() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert!(result.events.is_empty());
        assert!(result.todos.is_empty());
        assert!(result.journals.is_empty());
    }

    #[test]
    fn test_alarm_with_repeat() {
        let ics_content = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
SUMMARY:Repeating Reminder
BEGIN:VALARM
ACTION:DISPLAY
TRIGGER:-PT30M
DURATION:PT10M
REPEAT:3
END:VALARM
END:VEVENT
END:VCALENDAR";

        let temp_file = create_temp_ics(ics_content);
        let result = parse_ics(temp_file.path()).unwrap();

        assert_eq!(result.events.len(), 1);
        let alarm = &result.events[0].alarms[0];
        assert_eq!(alarm.duration, Some("PT10M".to_string()));
        assert_eq!(alarm.repeat, Some(3));
    }
}
