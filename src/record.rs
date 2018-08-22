use std::string::ToString;
use std::str::FromStr;
use regex::Regex;
use chrono::{DateTime, Local, Duration, TimeZone, Timelike};
use error::TimeTrackError;

lazy_static! {
    pub static ref RECORD_REGEX: Regex = {
        let regex_string = format!(
            r"^\[\s*(?P<{}>[^,]*),\s*(?P<{}>[0-9]*)\s*\(\s*(?P<{}>\-?[0-9]*)\s*\)\s*\]\s*(?P<{}>[^\n|^\r\n]*)\r?\n*$",
            RecordFieldName::Start.name(),
            RecordFieldName::Duration.name(),
            RecordFieldName::Correction.name(),
            RecordFieldName::Note.name()
        );
        Regex::new(&regex_string).unwrap()
    };
}

#[derive(Debug, Default, PartialEq, PartialOrd, FieldType, FieldName)]
pub struct Record {
    pub start: Option<DateTime<Local>>,
    pub duration: Option<Duration>,
    pub correction: Option<Duration>,
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
}

impl ToString for Record {
    fn to_string(&self) -> String {
        let line = format!(
            "[{}, {} ({})]",
            to_string_opt_as_str!(self.start.map(|dt| dt.format(Record::START_DATETIME_FORMAT))),
            to_string_opt_as_str!(self.duration.map(|d| d.num_minutes())),
            to_string_opt_as_str!(self.correction.map(|d| d.num_minutes()))
        );
        if !self.note.is_empty() {
            format!("{} {}\n", line, self.note)
        } else {
            line + "\n"
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
                duration: caps[RecordFieldName::Duration.name()].parse::<i64>()
                    .map(|min| Duration::minutes(min)).ok(),
                correction: caps[RecordFieldName::Correction.name()].parse::<i64>()
                    .map(|min| Duration::minutes(min)).ok(),
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
        assert!(RECORD_REGEX.is_match("[,()]"));
        assert!(RECORD_REGEX.is_match("[, ()]"));
        assert!(RECORD_REGEX.is_match("[,  ()] \n"));
        assert!(RECORD_REGEX.is_match("[2018-07-26 23:03:41, 25 (7)] Some note"));
        assert!(RECORD_REGEX.is_match("[2018-07-26 23:03:41, 25 (-16)] Some note"));

        let caps = RECORD_REGEX.captures("[,()]").unwrap();
        assert!(caps["start"].is_empty());
        assert!(caps["duration"].is_empty());
        assert!(caps["correction"].is_empty());
        assert!(caps["note"].is_empty());

        let caps = RECORD_REGEX.captures("[,  ()] \n").unwrap();
        assert!(caps["start"].is_empty());
        assert!(caps["duration"].is_empty());
        assert!(caps["correction"].is_empty());
        assert!(caps["note"].is_empty());

        let caps = RECORD_REGEX.captures("[2018-07-26 23:03:41,  ()] Some note").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert!(caps["duration"].is_empty());
        assert!(caps["correction"].is_empty());
        assert_eq!(&caps["note"], "Some note");

        let caps = RECORD_REGEX.captures("[2018-07-26 23:03:41, 25 (6)] Some note").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert_eq!(&caps["duration"], "25");
        assert_eq!(&caps["correction"], "6");
        assert_eq!(&caps["note"], "Some note");

        let caps = RECORD_REGEX.captures("[  2018-07-26 23:03:41,  25  ( -16 ) ]  Some note\n").unwrap();
        assert_eq!(&caps["start"], "2018-07-26 23:03:41");
        assert_eq!(&caps["duration"], "25");
        assert_eq!(&caps["correction"], "-16");
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
            duration: Some(Duration::minutes(33)),
            correction: Some(Duration::minutes(-5)),
            note: "Some note".to_string(),
        };

        let line = record.to_string();
        assert_eq!(record, line.parse::<Record>().unwrap());
    }
}