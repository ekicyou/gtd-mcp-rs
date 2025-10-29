use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Get the current date in local timezone
pub fn local_date_today() -> NaiveDate {
    Local::now().date_naive()
}

/// Recurrence pattern for recurring tasks
///
/// Defines how a task repeats after completion.
/// Uses snake_case naming to match TOML serialization format.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrencePattern {
    /// Repeats every day
    daily,
    /// Repeats on specific weekdays (e.g., Monday, Wednesday, Friday)
    weekly,
    /// Repeats on specific days of the month (e.g., 1st, 15th, 25th)
    monthly,
    /// Repeats on specific month-days each year (e.g., Jan 1, Dec 25)
    yearly,
}

/// Task status in the GTD workflow
///
/// Represents the different states a task can be in according to GTD methodology.
/// Uses snake_case naming to match TOML serialization format.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotaStatus {
    /// Unprocessed items
    inbox,
    /// Tasks to do now
    next_action,
    /// Tasks waiting for someone else or an external event
    waiting_for,
    /// Tasks to do later (not immediately actionable)
    later,
    /// Tasks scheduled for a specific date
    calendar,
    /// Tasks that might be done someday but not now
    someday,
    /// Completed tasks
    done,
    /// Reference material (non-actionable information for future reference)
    reference,
    /// Context nota (represents a location, tool, or situation)
    context,
    /// Project nota (represents a multi-step outcome)
    project,
    /// Deleted or discarded items
    trash,
}

impl FromStr for NotaStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(NotaStatus::inbox),
            "next_action" => Ok(NotaStatus::next_action),
            "waiting_for" => Ok(NotaStatus::waiting_for),
            "someday" => Ok(NotaStatus::someday),
            "later" => Ok(NotaStatus::later),
            "calendar" => Ok(NotaStatus::calendar),
            "done" => Ok(NotaStatus::done),
            "reference" => Ok(NotaStatus::reference),
            "trash" => Ok(NotaStatus::trash),
            "context" => Ok(NotaStatus::context),
            "project" => Ok(NotaStatus::project),
            _ => Err(format!(
                "Invalid status '{}'. Valid options are: inbox, next_action, waiting_for, someday, later, calendar, done, reference, trash, context, project",
                s
            )),
        }
    }
}

/// A unified nota (note) in the GTD system
///
/// Nota unifies Task, Project, and Context into a single structure.
/// The `status` field determines what type of nota it is:
/// - status = "context": represents a Context
/// - status = "project": represents a Project
/// - other statuses (inbox, next_action, etc.): represents a Task
///
/// This design is inspired by TiddlyWiki's tiddler concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Nota {
    /// Unique identifier (e.g., "meeting-prep", "website-redesign", "Office")
    pub id: String,
    /// Title describing the nota
    pub title: String,
    /// Current status (inbox, next_action, waiting_for, later, calendar, someday, done, trash, context, project)
    pub status: NotaStatus,
    /// Optional parent project ID
    pub project: Option<String>,
    /// Optional context where this nota applies
    pub context: Option<String>,
    /// Optional additional notes in Markdown format
    pub notes: Option<String>,
    /// Optional start date (format: YYYY-MM-DD)
    pub start_date: Option<NaiveDate>,
    /// Date when the nota was created
    pub created_at: NaiveDate,
    /// Date when the nota was last updated
    pub updated_at: NaiveDate,
    /// Optional recurrence pattern (daily, weekly, monthly, yearly)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_pattern: Option<RecurrencePattern>,
    /// Optional recurrence configuration (weekdays for weekly, dates for monthly/yearly)
    /// Format: comma-separated values
    /// - weekly: weekday names (e.g., "Monday,Wednesday,Friday")
    /// - monthly: day numbers (e.g., "1,15,25")
    /// - yearly: month-day pairs (e.g., "1-1,12-25" for Jan 1 and Dec 25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_config: Option<String>,
}

impl Default for Nota {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            recurrence_pattern: None,
            recurrence_config: None,
        }
    }
}

impl Nota {
    /// Check if this nota is a task
    pub fn is_task(&self) -> bool {
        !matches!(self.status, NotaStatus::context | NotaStatus::project)
    }

    /// Check if this nota is a project
    pub fn is_project(&self) -> bool {
        self.status == NotaStatus::project
    }

    /// Check if this nota is a context
    pub fn is_context(&self) -> bool {
        self.status == NotaStatus::context
    }

    /// Check if this nota has recurrence configured
    pub fn is_recurring(&self) -> bool {
        self.recurrence_pattern.is_some()
    }

    /// Calculate the next occurrence date for a recurring task
    ///
    /// # Arguments
    /// * `from_date` - The date to calculate from (typically the current start_date or today)
    ///
    /// # Returns
    /// The next occurrence date if this is a recurring task, None otherwise
    pub fn calculate_next_occurrence(&self, from_date: NaiveDate) -> Option<NaiveDate> {
        use chrono::{Datelike, Duration, Weekday};

        let pattern = self.recurrence_pattern.as_ref()?;
        let config = self.recurrence_config.as_ref();

        match pattern {
            RecurrencePattern::daily => Some(from_date + Duration::days(1)),

            RecurrencePattern::weekly => {
                let weekdays = config?;
                let target_weekdays: Vec<Weekday> = weekdays
                    .split(',')
                    .filter_map(|day| match day.trim() {
                        "Monday" => Some(Weekday::Mon),
                        "Tuesday" => Some(Weekday::Tue),
                        "Wednesday" => Some(Weekday::Wed),
                        "Thursday" => Some(Weekday::Thu),
                        "Friday" => Some(Weekday::Fri),
                        "Saturday" => Some(Weekday::Sat),
                        "Sunday" => Some(Weekday::Sun),
                        _ => None,
                    })
                    .collect();

                if target_weekdays.is_empty() {
                    return None;
                }

                // Find the next occurrence of any of the target weekdays
                let mut next_date = from_date + Duration::days(1);

                for _ in 0..7 {
                    if target_weekdays.contains(&next_date.weekday()) {
                        return Some(next_date);
                    }
                    next_date += Duration::days(1);
                }

                None
            }

            RecurrencePattern::monthly => {
                let days = config?;
                let target_days: Vec<u32> = days
                    .split(',')
                    .filter_map(|day| day.trim().parse::<u32>().ok())
                    .collect();

                if target_days.is_empty() {
                    return None;
                }

                // Find the next occurrence of any of the target days
                let mut next_date = from_date + Duration::days(1);
                for _ in 0..366 {
                    // Check up to 1 year ahead
                    if target_days.contains(&next_date.day()) {
                        return Some(next_date);
                    }
                    next_date += Duration::days(1);
                }

                None
            }

            RecurrencePattern::yearly => {
                let dates = config?;
                let target_dates: Vec<(u32, u32)> = dates
                    .split(',')
                    .filter_map(|date| {
                        let parts: Vec<&str> = date.trim().split('-').collect();
                        if parts.len() == 2 {
                            let month = parts[0].parse::<u32>().ok()?;
                            let day = parts[1].parse::<u32>().ok()?;
                            Some((month, day))
                        } else {
                            None
                        }
                    })
                    .collect();

                if target_dates.is_empty() {
                    return None;
                }

                // Find the next occurrence of any of the target dates
                let mut next_date = from_date + Duration::days(1);
                for _ in 0..366 {
                    // Check up to 1 year ahead
                    if target_dates.contains(&(next_date.month(), next_date.day())) {
                        return Some(next_date);
                    }
                    next_date += Duration::days(1);
                }

                None
            }
        }
    }
}
