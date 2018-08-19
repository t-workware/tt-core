use std::fs::OpenOptions;
use std::ffi::OsString;
use std::io::{Write, BufReader, BufWriter};
use std::str::FromStr;

use ropey::Rope;

use record::{Record, RecordFieldType};
use journal::{Journal, JournalResult};

pub trait LinesIterator: Iterator {
    fn lines_count(&self) -> usize;
    fn cur_line_idx(&self) -> Option<usize>;
    fn set_cur_line_idx(&mut self, idx: Option<usize>);

    fn rev_next(&mut self) -> Option<<Self as Iterator>::Item> {
        let idx = self.cur_line_idx()?;
        self.set_cur_line_idx(if idx > 1 {Some(idx - 2)} else {None});
        if idx > 0 {
            self.next()
        } else {
            None
        }
    }

    fn rev_skip_next(&mut self, skipped: usize) -> Option<<Self as Iterator>::Item> {
        if let Some(idx) = self.cur_line_idx() {
            self.set_cur_line_idx(
                if idx < skipped {
                    None
                } else {
                    Some(idx - skipped)
                }
            );
            self.rev_next()
        } else {
            None
        }
    }

    fn skip_next(&mut self, skipped: usize) -> Option<<Self as Iterator>::Item> {
        if skipped > 0 {
            let mut to_line_index = skipped;
            if let Some(idx) = self.cur_line_idx() {
                to_line_index += idx;
            } else {
                to_line_index -= 1;
            }

            let lines_count = self.lines_count();
            self.set_cur_line_idx(Some(
                if to_line_index < lines_count {
                    to_line_index
                } else {
                    lines_count
                }
            ));
        }
        self.next()
    }

    fn go_to_start(&mut self) {
        self.set_cur_line_idx(None);
    }

    fn go_to_end(&mut self) {
        let count = self.lines_count();
        self.set_cur_line_idx(if count > 0 {Some(count)} else {None});
    }
}

impl Iterator for Iter {
    type Item = Record;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let next_line_idx = self.cur_line_idx.map(|idx| idx + 1).unwrap_or(0);
        let lines_count = self.lines_count();
        if next_line_idx < lines_count {
            self.cur_line_idx = Some(next_line_idx);
            self.rope
                .line(next_line_idx)
                .as_str()
                .map(Record::from_str)
                .and_then(|res| res.ok())
        } else {
            self.cur_line_idx = Some(lines_count);
            None
        }
    }
}

impl LinesIterator for Iter {
    fn lines_count(&self) -> usize {
        let count = self.rope.len_lines();
        if count > 0 && self.rope.line(count - 1).as_str().map(str::is_empty).unwrap_or(true) {
            count - 1
        } else {
            count
        }
    }

    fn cur_line_idx(&self) -> Option<usize> {
        self.cur_line_idx
    }

    fn set_cur_line_idx(&mut self, idx: Option<usize>) {
        self.cur_line_idx = idx;
    }
}

#[derive(Default)]
pub struct Iter {
    path: OsString,
    rope: Rope,
    cur_line_idx: Option<usize>,
}

impl Iter {
    pub fn update_cur(&mut self, record: &Record) -> Option<usize> {
        let cur_line_idx = self.cur_line_idx?;
        let start_idx = self.rope.line_to_char(cur_line_idx);
        if cur_line_idx + 1 <= self.rope.len_lines() {
            let end_idx = self.rope.line_to_char(cur_line_idx + 1);
            self.rope.remove(start_idx..end_idx);
        }
        self.rope.insert(start_idx, &record.to_string());
        Some(start_idx)
    }

    pub fn flush(&mut self) -> JournalResult {
        let file = OpenOptions::new().write(true).open(&self.path)?;
        self.rope.write_to(BufWriter::new(file))?;
        Ok(())
    }
}

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
        Ok(Iter {
            path: self.path.clone(),
            rope,
            cur_line_idx: None,
        })
    }
}

impl Journal for FileJournal {
    fn add(&mut self, record: &Record) -> JournalResult {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        file.write(record.to_string().as_bytes())?;
        Ok(())
    }

    fn get(&self, query: &[RecordFieldType], offset: Option<i32>) -> JournalResult<Option<Record>> {
        let offset = offset.unwrap_or(0);
        let mut iter = self.try_iter()?;
        'next_record: while let Some(record) = iter.next() {
            for field in query.iter() {
                if !match field {
                    RecordFieldType::Start(x) => *x == record.start,
                    RecordFieldType::Duration(x) => *x == record.duration,
                    RecordFieldType::Correction(x) => *x == record.correction,
                    RecordFieldType::Note(x) => *x == record.note,
                } {
                    continue 'next_record;
                }
            }

            let cur_line_idx = iter.cur_line_idx()
                .expect("The current line index can't be None after next iteration");

            return Ok(
                if offset == 0 {
                    Some(record)
                } else {
                    let lines_count = iter.lines_count();

                    if offset > 0 && cur_line_idx + (offset as usize) < lines_count {
                        iter.skip((offset - 1) as usize).next()
                    } else if offset < 0 {
                        let start_idx = if cur_line_idx == 0 {
                            iter.go_to_end();
                            lines_count
                        } else {
                            cur_line_idx
                        };
                        if start_idx as i32 + offset >= 0 {
                            iter.rev_skip_next(-(offset + 1) as usize)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            );
        }
        Ok(None)
    }

    fn update<F>(&mut self, query: &[RecordFieldType], offset: Option<i32>, f: F) -> JournalResult
        where F: FnOnce(Record) -> Option<Record>,
    {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn record_with_note(note: &str) -> Record {
        Record {
            note: note.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn iter_walk_forward() {
        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\r\n[,()] bazz\n"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b""))).unwrap(),
            ..Default::default()
        };
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"\n"))).unwrap(),
            ..Default::default()
        };
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"\r\n"))).unwrap(),
            ..Default::default()
        };
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\r\n"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_walk_backward() {
        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert_eq!(record_with_note("bazz"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("bar"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        assert!(iter.rev_next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz\n"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert_eq!(record_with_note("bazz"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("bar"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        assert!(iter.rev_next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b""))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert!(iter.rev_next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"\n"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert!(iter.rev_next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"\r\n"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert!(iter.rev_next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\r\n"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_walk_combined() {
        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert!(iter.rev_next().is_none());
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        assert!(iter.rev_next().is_none());
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert!(iter.next().is_none());
        assert_eq!(record_with_note("bazz"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("bar"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        assert!(iter.rev_next().is_none());
    }

    #[test]
    fn iter_skip_forward() {
        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("bar"), iter.skip_next(1).unwrap());
        assert!(iter.skip_next(2).is_none());
        assert_eq!(record_with_note("bazz"), iter.rev_next().unwrap());
        iter.go_to_start();
        assert_eq!(record_with_note("bar"), iter.skip_next(1).unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        iter.go_to_start();
        assert_eq!(record_with_note("bazz"), iter.skip_next(2).unwrap());
        assert_eq!(record_with_note("bar"), iter.rev_next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.skip_next(0).unwrap());
        assert!(iter.skip_next(1).is_none());
    }

    #[test]
    fn iter_skip_backward() {
        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
            ..Default::default()
        };
        iter.go_to_end();
        assert_eq!(record_with_note("bar"), iter.rev_skip_next(1).unwrap());
        assert!(iter.rev_skip_next(2).is_none());
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        iter.go_to_end();
        assert_eq!(record_with_note("bar"), iter.rev_skip_next(1).unwrap());
        assert_eq!(record_with_note("foo"), iter.rev_next().unwrap());
        iter.go_to_end();
        assert_eq!(record_with_note("foo"), iter.rev_skip_next(2).unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("foo"), iter.rev_skip_next(0).unwrap());
        assert!(iter.rev_skip_next(1).is_none());
    }

    #[test]
    fn iter_update() {
        let mut iter = Iter {
            rope: Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
            ..Default::default()
        };
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        iter.update_cur(&record_with_note("test"));
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert!(iter.next().is_none());
        assert_eq!(record_with_note("test"), iter.rev_skip_next(1).unwrap());

        iter.go_to_start();
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        iter.update_cur(&record_with_note("note"));
        assert_eq!(record_with_note("bazz"), iter.skip_next(1).unwrap());
        iter.update_cur(&record_with_note("some"));

        iter.go_to_start();
        assert_eq!(record_with_note("note"), iter.next().unwrap());
        assert_eq!(record_with_note("test"), iter.next().unwrap());
        assert_eq!(record_with_note("some"), iter.next().unwrap());
        assert!(iter.next().is_none());
    }
}