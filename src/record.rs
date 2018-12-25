use std::string::ToString;
use std::str::FromStr;
use regex::Regex;
pub use chrono::{DateTime, Local, Duration, TimeZone, Timelike, Date, Datelike};
use error::TimeTrackError;
use lazy_static::lazy_static;
use field_types::{FieldType, FieldName};

lazy_static! {
    pub static ref RECORD_REGEX: Regex = {
        let regex_string = format!(
            r"^\[\s*(?P<{}>[^,]*),\s*(?P<{}>[0-9]*)\s*(?:\(\s*(?P<{}>\-?[0-9]*)\s*\))?\s*\]\s*(?P<{}>[^\n|^\r\n]*)\r?\n*$",
            RecordFieldName::Start.name(),
            RecordFieldName::Activity.name(),
            RecordFieldName::Rest.name(),
            RecordFieldName::Note.name()
        );
        Regex::new(&regex_string).unwrap()
    };
}

#[derive(Debug, Default, PartialEq, PartialOrd, FieldType, FieldName)]
pub struct Record {
    pub start: Option<DateTime<Local>>,
    pub activity: Option<Duration>,
    pub rest: Option<Duration>,
    pub note: String,
}

impl Record {
    pub const START_DATETIME_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn now() -> Self {
        let now = Local::now();
        Record {
            start: Some(now - Duration::nanoseconds(now.nanosecond() as i64)),
            ..Default::default()
        }
    }

    pub fn duration_until_now(&self) -> Duration {
        Duration::minutes(
            self.start
                .map(|start| (Local::now() - start).num_minutes())
                .unwrap_or(0)
        )
    }

    pub fn update_activity_to_now(&mut self) {
        if self.start.is_some() {
            self.activity = Some(self.duration_until_now() - self.rest.unwrap_or(Duration::minutes(0)));
        }
    }

    pub fn update_rest_to_now(&mut self) {
        if self.start.is_some() {
            self.rest = Some(self.duration_until_now() - self.activity.unwrap_or(Duration::minutes(0)));
        }
    }
}

impl ToString for Record {
    fn to_string(&self) -> String {
        let rest = if let Some(rest) = self.rest {
            format!(" ({})", rest.num_minutes())
        } else {
            String::new()
        };

        let timing = if let Some(activity) = self.activity {
            format!("{}{}", activity.num_minutes(), rest)
        } else {
            rest
        };

        let line = format!(
            "[{}, {}]",
            to_string_opt_as_str!(self.start.map(|dt| dt.format(Record::START_DATETIME_FORMAT))),
            timing
        );
        if !self.note.is_empty() {
            format!("{} {}", line, self.note)
        } else {
            line
        }
    }
}

impl FromStr for Record {
    type Err = TimeTrackError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = RECORD_REGEX.captures_iter(source).next() {
            Ok(Record {
                start: Local.datetime_from_str(&caps[RecordFieldName::Start.name()],
                                               Record::START_DATETIME_FORMAT).ok(),
                activity: caps[RecordFieldName::Activity.name()].parse::<i64>()
                    .map(|min| Duration::minutes(min)).ok(),
                rest: caps.name(RecordFieldName::Rest.name())
                    .and_then(|rest| rest.as_str().parse::<i64>().ok())
                    .map(|min| Duration::minutes(min)),
                note: caps[RecordFieldName::Note.name()].to_string(),
            })
        } else {
            Err(TimeTrackError::CanNotParseRecord { source: source.to_string() })
        }
    }
}


pub enum RecordQuery {
    Field(RecordFieldType),
    Offset(i32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_regex() {
        assert!(RECORD_REGEX.is_match("[,]"));
        assert!(RECORD_REGEX.is_match("[, ]"));
        assert!(RECORD_REGEX.is_match("[,()]"));
        assert!(RECORD_REGEX.is_match("[, ()]"));
        assert!(RECORD_REGEX.is_match("[,  ()] \n"));
        assert!(RECORD_REGEX.is_match("[, ()  ]"));
        assert!(RECORD_REGEX.is_match("[2018-07-26 23:03:41, ] Some note"));
        assert!(RECORD_REGEX.is_match("[2018-07-26 23:03:41, 25] Some note"));
        assert!(RECORD_REGEX.is_match("[2018-07-26 23:03:41, 25 ] Some note"));
        assert!(RECORD_REGEX.is_match("[2018-07-26 23:03:41, 25 (7)] Some note"));
        assert!(RECORD_REGEX.is_match("[2018-07-26 23:03:41, 25 (-16)] Some note"));

        let caps = RECORD_REGEX.captures("[,]").unwrap();
        assert!(caps["start"].is_empty());
        assert!(caps["activity"].is_empty());
        assert!(caps.name("rest").is_none());
        assert!(caps["note"].is_empty());

        let caps = RECORD_REGEX.captures("[,()]").unwrap();
        assert!(caps["start"].is_empty());
        assert!(caps["activity"].is_empty());
        assert!(caps["rest"].is_empty());
        assert!(caps["note"].is_empty());

        let caps = RECORD_REGEX.captures("[,  ()] \n").unwrap();
        assert!(caps["start"].is_empty());
        assert!(caps["activity"].is_empty());
        assert!(caps["rest"].is_empty());
        assert!(caps["note"].is_empty());

        let caps = RECORD_REGEX.captures("[2018-07-26 23:03:41, ] Some note").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert!(caps["activity"].is_empty());
        assert!(caps.name("rest").is_none());
        assert_eq!(&caps["note"], "Some note");

        let caps = RECORD_REGEX.captures("[2018-07-26 23:03:41,  ()] Some note").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert!(caps["activity"].is_empty());
        assert!(caps["rest"].is_empty());
        assert_eq!(&caps["note"], "Some note");

        let caps = RECORD_REGEX.captures("[2018-07-26 23:03:41, 25] Some note").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert_eq!(&caps["activity"], "25");
        assert!(caps.name("rest").is_none());
        assert_eq!(&caps["note"], "Some note");

        let caps = RECORD_REGEX.captures("[2018-07-26 23:03:41, 25 (6)] Some note").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert_eq!(&caps["activity"], "25");
        assert_eq!(&caps["rest"], "6");
        assert_eq!(&caps["note"], "Some note");

        let caps = RECORD_REGEX.captures("[  2018-07-26 23:03:41,  25  ( -16 ) ]  Some note\n").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert_eq!(&caps["activity"], "25");
        assert_eq!(&caps["rest"], "-16");
        assert_eq!(&caps["note"], "Some note");

        assert!(!RECORD_REGEX.is_match("[]"));
        assert!(!RECORD_REGEX.is_match(",()]"));
        assert!(!RECORD_REGEX.is_match("[,()"));
    }

    #[test]
    fn to_string_from_str() {
        let record = Record::default();
        let line = record.to_string();
        assert_eq!(record, line.parse::<Record>().unwrap());

        let now = Local::now();
        let now = now - Duration::nanoseconds(now.nanosecond() as i64);
        let record = Record {
            start: Some(now.clone()),
            activity: Some(Duration::minutes(33)),
            rest: Some(Duration::minutes(-5)),
            note: "Some note".to_string(),
        };

        let line = record.to_string();
        assert_eq!(record, line.parse::<Record>().unwrap());
    }

    #[test]
    fn set_activity_to_now() {
        let mut record = Record {
            start: Some(Local::now() - Duration::minutes(12)),
            ..Default::default()
        };
        record.update_activity_to_now();
        assert_eq!(record.activity.unwrap(), Duration::minutes(12));

        let mut record = Record {
            start: Some(Local::now() - Duration::minutes(42)),
            activity: Some(Duration::minutes(10)),
            rest: Some(Duration::minutes(12)),
            ..Default::default()
        };
        record.update_activity_to_now();
        assert_eq!(record.activity.unwrap(), Duration::minutes(30));
    }

    #[test]
    fn set_rest_to_now() {
        let mut record = Record {
            start: Some(Local::now() - Duration::minutes(42)),
            activity: Some(Duration::minutes(30)),
            ..Default::default()
        };
        record.update_rest_to_now();
        assert_eq!(record.rest.unwrap(), Duration::minutes(12));
    }
}