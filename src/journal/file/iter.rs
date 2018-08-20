use std::fs;
use std::ffi::OsString;
use std::io::BufWriter;
use std::str::FromStr;

use ropey::Rope;

use record::Record;
use journal::JournalResult;

#[derive(Default)]
pub struct Iter {
    path: OsString,
    rope: Rope,
    cur_line_idx: Option<usize>,
}

impl Iter {
    pub fn new(path: OsString, rope: Rope, cur_line_idx: Option<usize>) -> Self {
        Iter {
            path,
            rope,
            cur_line_idx,
        }
    }

    pub fn with_rope(mut self, rope: Rope) -> Self {
        self.rope = rope;
        self
    }

    pub fn lines_count(&self) -> usize {
        let count = self.rope.len_lines();
        if count > 0 && self.rope.line(count - 1).as_str().map(str::is_empty).unwrap_or(true) {
            count - 1
        } else {
            count
        }
    }

    pub fn get(&self) -> Option<<Self as Iterator>::Item> {
        if let Some(cur_line_idx) = self.cur_line_idx {
            if cur_line_idx < self.lines_count() {
                return self.rope
                    .line(cur_line_idx)
                    .as_str()
                    .map(Record::from_str)
                    .and_then(|res| res.ok());
            }
        }
        None
    }

    pub fn forward(&mut self, n: usize) -> &Self {
        if n > 0 {
            let to_line_idx = self.cur_line_idx.map(|idx| idx + n).unwrap_or(n - 1);
            let lines_count = self.lines_count();

            self.cur_line_idx = Some(
                if to_line_idx < lines_count {
                    to_line_idx
                } else {
                    lines_count
                }
            );
        }
        self
    }

    pub fn backward(&mut self, n: usize) -> &Self {
        if let Some(idx) = self.cur_line_idx {
            self.cur_line_idx = if idx < n { None } else { Some(idx - n) };
        }
        self
    }

    pub fn go_to_start(&mut self) {
        self.cur_line_idx = None;
    }

    pub fn go_to_end(&mut self) {
        let count = self.lines_count();
        self.cur_line_idx = if count > 0 {Some(count)} else {None};
    }

    pub fn update(&mut self, item: &<Self as Iterator>::Item) -> Option<usize> {
        let cur_line_idx = self.cur_line_idx?;
        let start_idx = self.rope.line_to_char(cur_line_idx);
        if cur_line_idx + 1 <= self.rope.len_lines() {
            let end_idx = self.rope.line_to_char(cur_line_idx + 1);
            self.rope.remove(start_idx..end_idx);
        }
        self.rope.insert(start_idx, &item.to_string());
        Some(start_idx)
    }

    pub fn flush(&mut self) -> JournalResult {
        let mut backup = self.path.clone();
        backup.push(".tt_back");

        fs::copy(&self.path, &backup)?;

        let file = fs::OpenOptions::new().truncate(true).write(true).open(&self.path)?;
        let result = if let Err(err) = self.rope.write_to(BufWriter::new(file)) {
            fs::copy(&backup, &self.path)?;
            Err(err.into())
        } else {
            Ok(())
        };

        fs::remove_file(&backup)?;
        result
    }
}

impl Iterator for Iter {
    type Item = Record;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.forward(1).get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    fn record_with_note(note: &str) -> Record {
        Record {
            note: note.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn iter_walk_next() {
        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap()
        );
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\r\n[,()] bazz\n"))).unwrap(),
        );
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b""))).unwrap(),
        );
        assert!(iter.next().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"\n"))).unwrap(),
        );
        assert!(iter.next().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"\r\n"))).unwrap(),
        );
        assert!(iter.next().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo"))).unwrap(),
        );
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n"))).unwrap(),
        );
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert!(iter.next().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\r\n"))).unwrap(),
        );
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_walk_forward() {
        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
        );
        assert_eq!(record_with_note("foo"), iter.forward(1).get().unwrap());
        assert_eq!(record_with_note("bazz"), iter.forward(2).get().unwrap());
        assert!(iter.forward(1).get().is_none());
        iter.go_to_start();
        iter.forward(1);
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("bazz"), iter.forward(1).get().unwrap());
        iter.go_to_start();
        assert_eq!(record_with_note("bar"), iter.forward(2).get().unwrap());
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        iter.go_to_start();
        assert_eq!(record_with_note("bazz"), iter.forward(3).get().unwrap());
        assert_eq!(record_with_note("bazz"), iter.forward(0).get().unwrap());
        assert!(iter.forward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b""))).unwrap(),
        );
        assert!(iter.forward(0).get().is_none());
        assert!(iter.forward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"\n"))).unwrap(),
        );
        assert!(iter.forward(0).get().is_none());
        assert!(iter.forward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"\r\n"))).unwrap(),
        );
        assert!(iter.forward(0).get().is_none());
        assert!(iter.forward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo"))).unwrap(),
        );
        assert!(iter.forward(0).get().is_none());
        assert_eq!(record_with_note("foo"), iter.forward(1).get().unwrap());
        assert!(iter.forward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n"))).unwrap(),
        );
        assert!(iter.forward(0).get().is_none());
        assert_eq!(record_with_note("foo"), iter.forward(1).get().unwrap());
        assert!(iter.forward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\r\n"))).unwrap(),
        );
        assert!(iter.forward(0).get().is_none());
        assert_eq!(record_with_note("foo"), iter.forward(1).get().unwrap());
        assert!(iter.forward(1).get().is_none());
    }

    #[test]
    fn iter_walk_backward() {
        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
        );
        iter.go_to_end();
        assert_eq!(record_with_note("bar"), iter.backward(2).get().unwrap());
        assert!(iter.backward(2).get().is_none());
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        iter.go_to_end();
        assert_eq!(record_with_note("bar"), iter.backward(2).get().unwrap());
        assert_eq!(record_with_note("foo"), iter.backward(1).get().unwrap());
        iter.go_to_end();
        assert_eq!(record_with_note("foo"), iter.backward(3).get().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.backward(0).get().unwrap());
        assert_eq!(record_with_note("foo"), iter.backward(1).get().unwrap());
        assert!(iter.backward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b""))).unwrap(),
        );
        iter.go_to_end();
        assert!(iter.backward(0).get().is_none());
        assert!(iter.backward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"\n"))).unwrap(),
        );
        iter.go_to_end();
        assert!(iter.backward(0).get().is_none());
        assert!(iter.backward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"\r\n"))).unwrap(),
        );
        iter.go_to_end();
        assert!(iter.backward(0).get().is_none());
        assert!(iter.backward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo"))).unwrap(),
        );
        iter.go_to_end();
        assert!(iter.backward(0).get().is_none());
        assert_eq!(record_with_note("foo"), iter.backward(1).get().unwrap());
        assert!(iter.backward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n"))).unwrap(),
        );
        iter.go_to_end();
        assert!(iter.backward(0).get().is_none());
        assert_eq!(record_with_note("foo"), iter.backward(1).get().unwrap());
        assert!(iter.backward(1).get().is_none());

        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\r\n"))).unwrap(),
        );
        iter.go_to_end();
        assert!(iter.backward(0).get().is_none());
        assert_eq!(record_with_note("foo"), iter.backward(1).get().unwrap());
        assert!(iter.backward(1).get().is_none());
    }

    #[test]
    fn iter_walk_combined() {
        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
        );
        assert_eq!(record_with_note("foo"), iter.forward(1).get().unwrap());
        assert!(iter.backward(1).get().is_none());
        assert_eq!(record_with_note("bar"), iter.forward(2).get().unwrap());
        assert_eq!(record_with_note("foo"), iter.backward(1).get().unwrap());
        assert!(iter.backward(1).get().is_none());
        assert_eq!(record_with_note("bazz"), iter.forward(3).get().unwrap());
        assert_eq!(record_with_note("foo"), iter.backward(2).get().unwrap());
        assert_eq!(record_with_note("bar"), iter.forward(1).get().unwrap());
        assert!(iter.forward(2).get().is_none());
        assert_eq!(record_with_note("bazz"), iter.backward(1).get().unwrap());
        assert!(iter.backward(3).get().is_none());
    }

    #[test]
    fn iter_update() {
        let mut iter = Iter::default().with_rope(
            Rope::from_reader(BufReader::new(Cursor::new(b"[,()] foo\n[,()] bar\n[,()] bazz"))).unwrap(),
        );
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        assert_eq!(record_with_note("bar"), iter.next().unwrap());
        iter.update(&record_with_note("test"));
        assert_eq!(record_with_note("bazz"), iter.next().unwrap());
        assert!(iter.next().is_none());
        assert_eq!(record_with_note("test"), iter.backward(2).get().unwrap());

        iter.go_to_start();
        assert_eq!(record_with_note("foo"), iter.next().unwrap());
        iter.update(&record_with_note("note"));
        assert_eq!(record_with_note("bazz"), iter.forward(2).get().unwrap());
        iter.update(&record_with_note("some"));

        iter.go_to_start();
        assert_eq!(record_with_note("note"), iter.next().unwrap());
        assert_eq!(record_with_note("test"), iter.next().unwrap());
        assert_eq!(record_with_note("some"), iter.next().unwrap());
        assert!(iter.next().is_none());
    }
}