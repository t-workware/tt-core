pub mod file;

use failure::Error;

use record::{Record, RecordFieldType};

pub type JournalResult<T = ()> = Result<T, Error>;

pub trait Journal {
    fn add(&mut self, record: &Record) -> JournalResult;
    fn get(&self, query: &[RecordFieldType], offset: Option<i32>) -> JournalResult<Option<Record>>;
    fn update(&mut self, query: &[RecordFieldType], offset: Option<i32>, record: &Record) -> JournalResult;
}