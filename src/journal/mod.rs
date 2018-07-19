pub mod file;

use record::Record;
use error::TimeTrackError;

pub trait Journal {
    fn add(&mut self, record: &Record) -> Result<(), TimeTrackError>;
    fn get(&self, search: &Record) -> Result<Record, TimeTrackError>;
    fn update(&mut self, search: &Record, record: &Record) -> Result<(), TimeTrackError>;
}