pub mod file;

use failure::Error;

use record::{Record, RecordQuery};

pub type JournalResult<T = ()> = Result<T, Error>;

pub trait Journal {
    fn add(&mut self, record: &Record) -> JournalResult;
    fn get(&self, query: &[RecordQuery]) -> JournalResult<Option<Record>>;
    fn update(&mut self, query: &[RecordQuery], record: &Record) -> JournalResult;
}