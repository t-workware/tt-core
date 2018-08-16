use std::fs::OpenOptions;
use std::ffi::OsString;
use std::io::{Write, BufReader, BufRead};

use record::{Record, RecordFieldType};
use journal::{Journal, JournalResult};

pub struct FileJournal {
    path: OsString,
}

impl FileJournal {
    pub fn new<P: Into<OsString>>(path: P) -> Self {
        FileJournal {
            path: path.into(),
        }
    }
}

impl Journal for FileJournal {
    fn add(&mut self, record: &Record) -> JournalResult {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        file.write(record.to_string().as_bytes())?;
        Ok(())
    }

    fn get(&self, query: &[RecordFieldType], offset: Option<i32>) -> JournalResult<Option<Record>> {
        unimplemented!()
    }

    fn update(&mut self, query: &[RecordFieldType], offset: Option<i32>, record: &Record) -> JournalResult {
        unimplemented!()
    }
}