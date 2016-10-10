//! Util functions

use std::i64;
use std::num::ParseIntError;

pub fn uuid_to_decimal<'a>(uuid: &'a str) -> Result<i64, ParseIntError> {
    let suffix = &uuid[24..];
    i64::from_str_radix(suffix, 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_to_decimal_okay() -> () {
        let uuid = "d6a51a3a-b378-11e4-924b-23b6ec126a13";

        let decimal = uuid_to_decimal(uuid).unwrap();

        assert_eq!(decimal, 39268551649811)
    }
}
