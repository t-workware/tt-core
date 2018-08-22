use std::fs::OpenOptions;
use std::ffi::OsString;
use std::io::{Write, BufReader};

use ropey::Rope;

use record::{Record, RecordFieldType};
use journal::{Journal, JournalResult};

mod iter;
pub use self::iter::*;

pub struct FileJournal {
    path: OsString,
}

impl FileJournal {
    pub fn new<P: Into<OsString>>(path: P) -> Self {
        FileJournal {
            path: path.into(),
        }
    }

    pub fn try_iter(&self) -> JournalResult<Iter> {
        let file = OpenOptions::new().read(true).open(&self.path)?;
        let rope = Rope::from_reader(BufReader::new(file))?;
        Ok(Iter::new(self.path.clone(), rope, None))
    }
}

impl Journal for FileJournal {
    fn add(&mut self, record: &Record) -> JournalResult {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        file.write((record.to_string() + "\n").as_bytes())?;
        Ok(())
    }

    fn get(&self, query: &[RecordFieldType], offset: Option<i32>) -> JournalResult<Option<Record>> {
        let mut iter = self.try_iter()?;
        Ok(iter_to_record(&mut iter, query, offset)?)
    }

    fn update<F>(&mut self, query: &[RecordFieldType], offset: Option<i32>, f: F) -> JournalResult<bool>
        where F: FnOnce(Record) -> Option<Record>,
    {
        let mut iter = self.try_iter()?;
        let updated = iter_to_record(&mut iter, query, offset)?
            .and_then(f)
            .map(|new_record| iter.update(&new_record).is_some())
            .unwrap_or(false);
        if updated {
            iter.flush()?;
        }
        Ok(updated)
    }
}

fn iter_to_record(iter: &mut Iter, query: &[RecordFieldType], offset: Option<i32>) -> JournalResult<Option<Record>> {
    let offset = offset.unwrap_or(0);
    let mut first_line = true;
    'next_record: while let Some(record) = iter.next() {
        for field in query {
            if !match field {
                RecordFieldType::Start(x) => *x == record.start,
                RecordFieldType::Duration(x) => *x == record.duration,
                RecordFieldType::Correction(x) => *x == record.correction,
                RecordFieldType::Note(x) => *x == record.note,
            } {
                first_line = false;
                continue 'next_record;
            }
        }

        return Ok(
            if offset == 0 {
                Some(record)
            } else if offset > 0 {
                iter.forward(offset as usize).get()
            } else {
                if first_line {
                    iter.go_to_end();
                }
                iter.backward(-offset as usize).get()
            }
        );
    }
    Ok(None)
}