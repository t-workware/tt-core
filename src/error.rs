use failure::Fail;

#[derive(Debug, PartialEq, PartialOrd, Fail)]
pub enum TimeTrackError {
    #[fail(display = "can't parse record from source: `{}`", source)]
    CanNotParseRecord {
        source: String,
    },
}