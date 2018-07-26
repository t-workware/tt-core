use chrono::{DateTime, Local, Duration};

#[derive(Debug, FieldsEnum, Default)]
pub struct Record {
    pub start: Option<DateTime<Local>>,
    pub duration: Option<Duration>,
    pub correction: Option<Duration>,
    pub note: Option<String>,
    pub offset: Option<i32>,
}
