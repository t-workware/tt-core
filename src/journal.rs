pub mod file;

use failure::Error;
use crate::record::{Record, RecordFieldType};

pub type JournalResult<T = ()> = Result<T, Error>;

pub trait Journal {
    fn add(&mut self, record: &Record) -> JournalResult;
    fn get(&self, query: &[RecordFieldType], offset: Option<i32>) -> JournalResult<Option<Record>>;
    fn update<F>(&mut self, query: &[RecordFieldType], offset: Option<i32>, f: F) -> JournalResult<bool>
        where F: FnOnce(Record) -> Option<Record>;
    fn remove<F>(&mut self, query: &[RecordFieldType], offset: Option<i32>, f: F) -> JournalResult<bool>
        where F: FnOnce(Record) -> bool;
}