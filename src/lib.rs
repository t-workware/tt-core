extern crate chrono;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate tt_derive;

pub mod journal;
pub mod error;
pub mod record;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
