use std::fs::OpenOptions;
use std::ffi::OsString;
use std::io::Write;

use record::{Record, RecordField};
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
        file.write_fmt(format_args!(
            "[{}, {} ({})] {}\n",
            to_string_opt_as_str!(record.start),
            to_string_opt_as_str!(record.duration),
            to_string_opt_as_str!(record.correction),
            string_opt_as_str!(record.note)
        ))?;
        Ok(())
    }

    fn get(&self, search: &[RecordField]) -> JournalResult<Record> {
        unimplemented!()
    }

    fn update(&mut self, search: &[RecordField], record: &Record) -> JournalResult {
        unimplemented!()
    }
}