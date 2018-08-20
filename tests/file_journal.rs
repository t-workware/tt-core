extern crate tt_core;
extern crate chrono;

#[macro_use]
mod common;

use std::path::PathBuf;
use chrono::{Local, Duration, TimeZone};
use tt_core::{
    record::{
        Record,
        RecordFieldType,
    },
    journal::{
        Journal,
        file::FileJournal,
    },
};

#[test]
fn add_record() {
    let journal_dir = &["..", "target", "test_file_journal", "add"].iter().collect::<PathBuf>();
    let journal_file = &journal_dir.join("journal.txt");
    clear_dir!(journal_dir);
    let mut journal = FileJournal::new(journal_file);
    let mut record = Record::default();

    journal.add(&record).expect("Can't add record to journal");
    assert_content!(journal_file, "[,  ()]\n");

    record.note = "Some note".to_string();
    journal.add(&record).expect("Can't add record to journal");
    assert_content!(journal_file, "[,  ()]\n[,  ()] Some note\n");

    let now = Local::now();
    let formatted_now = now.format(Record::START_DATETIME_FORMAT).to_string();
    record.start = Some(now.clone());
    journal.add(&record).expect("Can't add record to journal");
    let expected = format!("[,  ()]\n[,  ()] Some note\n[{},  ()] Some note\n", formatted_now);
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

#[test]
fn get_record() {
    let journal_dir = &["..", "target", "test_file_journal", "get"].iter().collect::<PathBuf>();
    let journal_file = &journal_dir.join("journal.txt");
    clear_dir!(journal_dir);
    let journal = FileJournal::new(journal_file);

    create_file!(journal_file, r"[2018-08-16 13:52:43, 42 (1)] Note 1
[2018-08-16 15:40:25, 42 (-5)] Note 2
[2018-08-16 18:12:01, 85 ()] Note 3
[2018-08-16 18:12:01, 85 ()]
");
    let expected_records = [
        Some(Record {
            start: Local.datetime_from_str("2018-08-16 13:52:43", Record::START_DATETIME_FORMAT).ok(),
            duration: Some(Duration::minutes(42)),
            correction: Some(Duration::minutes(1)),
            note: "Note 1".to_string(),
        }),
        Some(Record {
            start: Local.datetime_from_str("2018-08-16 15:40:25", Record::START_DATETIME_FORMAT).ok(),
            duration: Some(Duration::minutes(42)),
            correction: Some(Duration::minutes(-5)),
            note: "Note 2".to_string(),
        }),
        Some(Record {
            start: Local.datetime_from_str("2018-08-16 18:12:01", Record::START_DATETIME_FORMAT).ok(),
            duration: Some(Duration::minutes(85)),
            correction: None,
            note: "Note 3".to_string(),
        }),
        Some(Record {
            start: Local.datetime_from_str("2018-08-16 18:12:01", Record::START_DATETIME_FORMAT).ok(),
            duration: Some(Duration::minutes(85)),
            correction: None,
            note: "".to_string(),
        })
    ];

    for i in 0..4 {
        let index = i as usize;
        let record = journal.get(&[], Some(i))
            .expect("Can't get record from journal");
        assert_eq!(expected_records[index], record);

        let record = journal.get(&[RecordFieldType::Correction(Some(Duration::minutes(-5)))], Some(i - 1))
            .expect("Can't get record from journal");
        assert_eq!(expected_records[index], record);

        let record = journal.get(&[RecordFieldType::Correction(None)], Some(i - 2))
            .expect("Can't get record from journal");
        assert_eq!(expected_records[index], record);

        let record = journal.get(&[RecordFieldType::Note("".to_string())], Some(i - 3))
            .expect("Can't get record from journal");
        assert_eq!(expected_records[index], record);

        let record = journal.get(&[], Some(i - 4))
            .expect("Can't get record from journal");
        assert_eq!(expected_records[index], record);
    }

    let record = journal.get(&[], None)
        .expect("Can't get record from journal");
    assert_eq!(expected_records[0], record);

    let record = journal.get(&[RecordFieldType::Duration(Some(Duration::minutes(42)))], Some(0))
        .expect("Can't get record from journal");
    assert_eq!(expected_records[0], record);

    let record = journal.get(&[RecordFieldType::Duration(Some(Duration::minutes(85)))], None)
        .expect("Can't get record from journal");
    assert_eq!(expected_records[2], record);

    let record = journal.get(&[
        RecordFieldType::Duration(Some(Duration::minutes(85))),
        RecordFieldType::Note("".to_string())
    ], None).expect("Can't get record from journal");
    assert_eq!(expected_records[3], record);
}

#[test]
fn update_record() {
    let journal_dir = &["..", "target", "test_file_journal", "update"].iter().collect::<PathBuf>();
    let journal_file = &journal_dir.join("journal.txt");
    clear_dir!(journal_dir);
    let mut journal = FileJournal::new(journal_file);

    create_file!(journal_file, r"[2018-08-16 13:52:43, 42 (1)] Note 1
[2018-08-16 15:40:25, 42 (-5)] Note 2
[2018-08-16 18:12:01, 85 ()] Note 3
[2018-08-16 18:12:01, 85 ()]
");

    assert!(journal.update(&[], Some(-1), |mut record| {
        record.start = Local.datetime_from_str("2018-08-20 22:30:15", Record::START_DATETIME_FORMAT).ok();
        record.note = "Note 4".to_string();
        Some(record)
    }).unwrap());
    assert!(journal.update(&[RecordFieldType::Correction(Some(Duration::minutes(1)))], None, |mut record| {
        record.start = Local.datetime_from_str("2018-08-20 22:40:12", Record::START_DATETIME_FORMAT).ok();
        record.duration = Some(Duration::minutes(12));
        Some(record)
    }).unwrap());
    assert!(journal.update(&[RecordFieldType::Correction(None)], None, |mut record| {
        record.correction = Some(Duration::minutes(-17));
        Some(record)
    }).unwrap());
    assert!(journal.update(&[RecordFieldType::Correction(Some(Duration::minutes(-17)))], Some(-1), |mut record| {
        record.correction = None;
        record.note = "".to_string();
        Some(record)
    }).unwrap());
    assert_content!(journal_file, r"[2018-08-20 22:40:12, 12 (1)] Note 1
[2018-08-16 15:40:25, 42 ()]
[2018-08-16 18:12:01, 85 (-17)] Note 3
[2018-08-20 22:30:15, 85 ()] Note 4
");
}