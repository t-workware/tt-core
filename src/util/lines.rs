use std::io::{Seek, SeekFrom, Result, BufRead};
use std::iter::Iterator;

pub trait LinesWalk: Iterator {
    fn rev_next(&mut self) -> Option<Result<String>>;
    fn rev_skip_next(&mut self, skipped: usize) -> Option<Result<String>>;
    fn go_to_end(&mut self) -> Result<u64>;
}

pub struct LinesWalker<R: BufRead + Seek> {
    reader: R,
}

impl<R: BufRead + Seek> LinesWalker<R> {
    pub fn new(reader: R) -> Self {
        LinesWalker {
            reader,
        }
    }

    #[inline]
    fn read_prev_byte(&mut self) -> Result<u8> {
        let mut byte = [0; 1];
        self.reader.seek(SeekFrom::Current(-1))?;
        let readed = self.reader.read(&mut byte)?;
        debug_assert_eq!(1, readed);
        Ok(byte[0])
    }

    #[inline]
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        let readed = self.reader.read_line(buf)?;
        if buf.ends_with("\n") {
            buf.pop();
            if buf.ends_with("\r") {
                buf.pop();
            }
        }
        Ok(readed)
    }
}

impl<R: BufRead + Seek> Iterator for LinesWalker<R> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = String::new();
        match self.read_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => Some(Ok(buf)),
            Err(err) => Some(Err(err)),
        }
    }
}

impl<R: BufRead + Seek> LinesWalk for LinesWalker<R> {
    fn rev_next(&mut self) -> Option<Result<String>> {
        self.rev_skip_next(0)
    }

    fn rev_skip_next(&mut self, skipped: usize) -> Option<Result<String>> {
        let mut prev_line = String::new();
        let mut pos = match self.reader.seek(SeekFrom::Current(0)) {
            Ok(pos) => pos,
            Err(err) => return Some(Err(err)),
        };

        for i in (0 .. skipped + 1).rev() {
            if pos == 0 {
                return None;
            }

            let mut read_process = || {
                if self.read_prev_byte()? == b'\n' {
                    pos = self.reader.seek(SeekFrom::Current(-1))?;
                    if pos > 0 && self.read_prev_byte()? == b'\r' {
                        pos = self.reader.seek(SeekFrom::Current(-1))?;
                    }
                }

                if pos > 0 {
                    loop {
                        if pos == 0 || self.read_prev_byte()? == b'\n' {
                            if i == 0 {
                                self.read_line(&mut prev_line)?;
                                self.reader.seek(SeekFrom::Start(pos))?;
                            }
                            break;
                        }
                        pos = self.reader.seek(SeekFrom::Current(-1))?;
                    }
                }
                Ok(())
            };

            if let Err(err) = read_process() {
                return Some(Err(err));
            }
        }

        Some(Ok(prev_line))
    }

    fn go_to_end(&mut self) -> Result<u64> {
        self.reader.seek(SeekFrom::End(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    #[test]
    fn walk_forward() {
        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\nbar\nbazz"))
        ).into_iter();
        assert_eq!("foo", iter.next().unwrap().unwrap());
        assert_eq!("bar", iter.next().unwrap().unwrap());
        assert_eq!("bazz", iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());

        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\nbar\r\nbazz\n"))
        ).into_iter();
        assert_eq!("foo", iter.next().unwrap().unwrap());
        assert_eq!("bar", iter.next().unwrap().unwrap());
        assert_eq!("bazz", iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());

        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b""))
        ).into_iter();
        assert!(iter.next().is_none());

        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b"\n"))
        ).into_iter();
        assert_eq!("", iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());

        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b"\r\n"))
        ).into_iter();
        assert_eq!("", iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());

        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo"))
        ).into_iter();
        assert_eq!("foo", iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());

        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\n"))
        ).into_iter();
        assert_eq!("foo", iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());

        let mut iter = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\r\n"))
        ).into_iter();
        assert_eq!("foo", iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());
    }

    #[test]
    fn walk_backward() {
        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\nbar\nbazz"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("bazz", walker.rev_next().unwrap().unwrap());
        assert_eq!("bar", walker.rev_next().unwrap().unwrap());
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());

        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\nbar\r\nbazz\n"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("bazz", walker.rev_next().unwrap().unwrap());
        assert_eq!("bar", walker.rev_next().unwrap().unwrap());
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());

        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b""))
        );
        walker.go_to_end().unwrap();
        assert!(walker.rev_next().is_none());

        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"\n"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());

        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"\r\n"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());

        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());

        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\n"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());

        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\r\n"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());
    }

    #[test]
    fn walk_combined() {
        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\nbar\nbazz"))
        );
        assert_eq!("foo", walker.next().unwrap().unwrap());
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());
        assert_eq!("foo", walker.next().unwrap().unwrap());
        assert_eq!("bar", walker.next().unwrap().unwrap());
        assert_eq!("bar", walker.rev_next().unwrap().unwrap());
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());
        assert_eq!("foo", walker.next().unwrap().unwrap());
        assert_eq!("bar", walker.next().unwrap().unwrap());
        assert_eq!("bazz", walker.next().unwrap().unwrap());
        assert_eq!("bazz", walker.rev_next().unwrap().unwrap());
        assert_eq!("bar", walker.rev_next().unwrap().unwrap());
        assert_eq!("bar", walker.next().unwrap().unwrap());
        assert_eq!("bazz", walker.next().unwrap().unwrap());
        assert!(walker.next().is_none());
        assert_eq!("bazz", walker.rev_next().unwrap().unwrap());
        assert_eq!("bar", walker.rev_next().unwrap().unwrap());
        assert_eq!("foo", walker.rev_next().unwrap().unwrap());
        assert!(walker.rev_next().is_none());
    }

    #[test]
    fn skip_backward() {
        let mut walker = LinesWalker::new(
            BufReader::new(Cursor::new(b"foo\nbar\nbazz"))
        );
        walker.go_to_end().unwrap();
        assert_eq!("bar", walker.rev_skip_next(1).unwrap().unwrap());
        assert!(walker.rev_skip_next(2).is_none());

        walker.go_to_end().unwrap();
        assert_eq!("foo", walker.rev_skip_next(2).unwrap().unwrap());
        assert_eq!("foo", walker.next().unwrap().unwrap());
        assert_eq!("foo", walker.rev_skip_next(0).unwrap().unwrap());
        assert!(walker.rev_skip_next(1).is_none());
    }
}