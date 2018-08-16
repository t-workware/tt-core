extern crate tt_core;
extern crate chrono;

#[macro_use]
mod common;

use std::path::PathBuf;
use chrono::{Local, Duration};
use tt_core::{
    record::Record,
    journal::{
        Journal,
        file::FileJournal,
    },
};

#[test]
fn add_record() {
    let journal_dir = &["..", "target", "test_file_journal"].iter().collect::<PathBuf>();
    let journal_file = &journal_dir.join("journal.txt");
    clear_dir!(journal_dir);
    let mut journal = FileJournal::new(journal_file);
    let mut record = Record::default();

    journal.add(&record).expect("Can't add record to journal");
    assert_content!(journal_file, "[,  ()] \n");

    record.note = "Some note".to_string();
    journal.add(&record).expect("Can't add record to journal");
    assert_content!(journal_file, "[,  ()] \n[,  ()] Some note\n");

    let now = Local::now();
    let formatted_now = now.format(Record::START_DATETIME_FORMAT).to_string();
    record.start = Some(now.clone());
    journal.add(&record).expect("Can't add record to journal");
    let expected = format!("[,  ()] \n[,  ()] Some note\n[{},  ()] Some note\n", formatted_now);
    assert_content!(journal_file, expected);

    delete_file!(journal_file);

    let duration = Duration::seconds(2533);
    let correction = Duration::seconds(2600) - duration;
    record.duration = Some(duration);
    record.correction = Some(correction);
    journal.add(&record).expect("Can't add record to journal");
    let expected = format!("[{}, 42 (1)] Some note\n", formatted_now);
    assert_content!(journal_file, expected);

    record.correction = Some(-correction);
    journal.add(&record).expect("Can't add record to journal");
    let expected = format!("[{}, 42 (1)] Some note\n[{}, 42 (-1)] Some note\n",
                           formatted_now, formatted_now);
    assert_content!(journal_file, expected);
}