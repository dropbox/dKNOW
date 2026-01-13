//! # docling-calendar
//!
//! iCalendar (ICS) file format parser for docling-rs.
//!
//! This crate provides parsing support for iCalendar files, the standard format
//! for calendar data interchange used by Google Calendar, Apple Calendar, Outlook,
//! and other calendar applications.
//!
//! ## Supported Format
//!
//! | Format | Extension | Description |
//! |--------|-----------|-------------|
//! | iCalendar | `.ics`, `.ical` | RFC 5545 calendar data format |
//!
//! ## What is iCalendar?
//!
//! iCalendar (RFC 5545) is a standard format for exchanging calendar and scheduling
//! information. ICS files can contain:
//!
//! - **VEVENT** - Calendar events (meetings, appointments)
//! - **VTODO** - Tasks and to-do items
//! - **VJOURNAL** - Journal entries and notes
//! - **VALARM** - Reminders and notifications
//!
//! ## Quick Start
//!
//! ### Parse Events from an ICS File
//!
//! ```no_run
//! use docling_calendar::parse_ics;
//!
//! let calendar = parse_ics("schedule.ics")?;
//!
//! // Calendar metadata
//! println!("Calendar: {}", calendar.name.unwrap_or_default());
//! println!("Timezone: {:?}", calendar.timezone);
//!
//! // List all events
//! for event in &calendar.events {
//!     println!("Event: {}", event.summary);
//!     println!("  Start: {:?}", event.start);
//!     println!("  End: {:?}", event.end);
//!     println!("  Location: {:?}", event.location);
//! }
//! # Ok::<(), docling_calendar::CalendarError>(())
//! ```
//!
//! ### Access Event Details
//!
//! ```no_run
//! use docling_calendar::parse_ics;
//!
//! let calendar = parse_ics("meeting.ics")?;
//!
//! for event in &calendar.events {
//!     // Basic info
//!     println!("Title: {}", event.summary);
//!     println!("Description: {:?}", event.description);
//!     println!("Location: {:?}", event.location);
//!
//!     // Time info
//!     println!("Start: {:?}", event.start);
//!     println!("End: {:?}", event.end);
//!
//!     // Participants
//!     println!("Organizer: {:?}", event.organizer);
//!     println!("Attendees: {:?}", event.attendees);
//!
//!     // Recurrence
//!     if let Some(rrule) = &event.recurrence {
//!         println!("Repeats: {}", rrule);
//!     }
//!
//!     // Alarms
//!     for alarm in &event.alarms {
//!         println!("Alarm: {:?} at {:?}", alarm.action, alarm.trigger);
//!     }
//! }
//! # Ok::<(), docling_calendar::CalendarError>(())
//! ```
//!
//! ### Parse Tasks (VTODOs)
//!
//! ```no_run
//! use docling_calendar::parse_ics;
//!
//! let calendar = parse_ics("tasks.ics")?;
//!
//! for todo in &calendar.todos {
//!     println!("Task: {}", todo.summary);
//!     println!("  Due: {:?}", todo.due);
//!     println!("  Priority: {:?}", todo.priority);
//!     println!("  Status: {:?}", todo.status);
//!     println!("  Progress: {:?}%", todo.percent_complete);
//! }
//! # Ok::<(), docling_calendar::CalendarError>(())
//! ```
//!
//! ### Parse Journal Entries
//!
//! ```no_run
//! use docling_calendar::parse_ics;
//!
//! let calendar = parse_ics("notes.ics")?;
//!
//! for journal in &calendar.journals {
//!     println!("Entry: {}", journal.summary);
//!     println!("  Date: {:?}", journal.date);
//!     println!("  Content: {:?}", journal.description);
//! }
//! # Ok::<(), docling_calendar::CalendarError>(())
//! ```
//!
//! ## Calendar Components
//!
//! ### Events (VEVENT)
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `summary` | `String` | Event title |
//! | `uid` | `Option<String>` | Unique identifier |
//! | `description` | `Option<String>` | Event description |
//! | `location` | `Option<String>` | Event location |
//! | `start` | `Option<String>` | Start date/time (DTSTART) |
//! | `end` | `Option<String>` | End date/time (DTEND) |
//! | `organizer` | `Option<String>` | Event organizer |
//! | `attendees` | `Vec<String>` | List of attendees |
//! | `status` | `Option<String>` | CONFIRMED, TENTATIVE, CANCELLED |
//! | `recurrence` | `Option<String>` | Recurrence rule (RRULE) |
//! | `alarms` | `Vec<CalendarAlarm>` | Reminders |
//!
//! ### Tasks (VTODO)
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `summary` | `String` | Task title |
//! | `description` | `Option<String>` | Task description |
//! | `due` | `Option<String>` | Due date |
//! | `priority` | `Option<u8>` | Priority (1=highest, 9=lowest) |
//! | `status` | `Option<String>` | NEEDS-ACTION, IN-PROCESS, COMPLETED |
//! | `percent_complete` | `Option<u8>` | Completion percentage (0-100) |
//!
//! ### Alarms (VALARM)
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `action` | `Option<String>` | DISPLAY, AUDIO, EMAIL |
//! | `trigger` | `Option<String>` | When to trigger (e.g., "-PT15M") |
//! | `description` | `Option<String>` | Alarm message |
//! | `duration` | `Option<String>` | Repeat interval |
//! | `repeat` | `Option<u32>` | Number of repetitions |
//!
//! ## Date/Time Format
//!
//! iCalendar dates are returned as strings in their original format:
//!
//! - **Date only**: `20251208` (December 8, 2025)
//! - **Date-time (UTC)**: `20251208T140000Z` (2:00 PM UTC)
//! - **Date-time (local)**: `20251208T140000` (2:00 PM local)
//! - **Date-time (with TZ)**: `TZID=America/New_York:20251208T140000`
//!
//! ## Recurrence Rules (RRULE)
//!
//! Events can have recurrence rules specifying how they repeat:
//!
//! - `FREQ=DAILY` - Every day
//! - `FREQ=WEEKLY;BYDAY=MO,WE,FR` - Monday, Wednesday, Friday
//! - `FREQ=MONTHLY;BYMONTHDAY=15` - 15th of each month
//! - `FREQ=YEARLY` - Once per year
//!
//! ## Use Cases
//!
//! - **Calendar aggregation**: Combine events from multiple sources
//! - **Event extraction**: Extract meeting details for reports
//! - **Task management**: Parse task lists from calendar apps
//! - **Scheduling analysis**: Analyze meeting patterns and availability
//!
//! ## Error Handling
//!
//! The parser returns `CalendarError` for:
//!
//! - File not found or unreadable
//! - Invalid ICS format
//! - Malformed calendar components
//!
//! ```no_run
//! use docling_calendar::{parse_ics, CalendarError};
//!
//! match parse_ics("calendar.ics") {
//!     Ok(cal) => println!("Found {} events", cal.events.len()),
//!     Err(CalendarError::ReadError { path, .. }) => {
//!         println!("Could not read file: {}", path.display());
//!     }
//!     Err(e) => println!("Parse error: {}", e),
//! }
//! ```

pub mod error;
pub mod ics;

pub use error::{CalendarError, Result};
pub use ics::{parse_ics, CalendarAlarm, CalendarEvent, CalendarJournal, CalendarTodo, IcsInfo};
