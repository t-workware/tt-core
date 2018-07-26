pub mod file;

use failure::Error;

use record::{Record, RecordField};

pub type JournalResult<T = ()> = Result<T, Error>;

pub trait Journal {
    fn add(&mut self, record: &Record) -> JournalResult;
    fn get(&self, search: &[RecordField]) -> JournalResult<Record>;
    fn update(&mut self, search: &[RecordField], record: &Record) -> JournalResult;
}