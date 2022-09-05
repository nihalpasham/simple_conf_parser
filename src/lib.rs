#![no_std]
#![allow(warnings)]

use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{alpha1, alphanumeric1, digit0, multispace0},
    combinator::opt,
    error::ErrorKind,
    sequence::{preceded, separated_pair, tuple},
    AsChar, Err, Err as NomErr, IResult, InputTakeAtPosition,
};

use core::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub struct ActiveConf<'a> {
    active_config: ConfigKeys,
    image_name: ImageLabel<'a>,
    image_version: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PassiveConf<'a> {
    passive_config: ConfigKeys,
    ready_for_update_flag: bool,
    image_name: Option<ImageLabel<'a>>,
    image_version: Option<u32>,
    update_status: Option<UpdateStatus>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigKeys {
    Active,
    Passive,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    Updating,
    Testing,
    Success,
}

pub type ImageLabel<'a> = (&'a str, &'a str);

impl From<&str> for ConfigKeys {
    fn from(i: &str) -> Self {
        match i {
            "[active]" => ConfigKeys::Active,
            "[passive]" => ConfigKeys::Passive,
            _ => unimplemented!("no other image types supported"),
        }
    }
}

impl From<&str> for UpdateStatus {
    fn from(i: &str) -> Self {
        match i {
            "Updating" => UpdateStatus::Updating,
            "Testing" => UpdateStatus::Testing,
            "Success" => UpdateStatus::Success,
            _ => unreachable!("invalid image state was set"),
        }
    }
}

fn config_keys(input: &str) -> IResult<&str, ConfigKeys> {
    alt((tag("[active]"), tag("[passive]")))(input)
        .map(|(next_input, res)| (next_input, res.into()))
}

fn image_name(input: &str) -> IResult<&str, ImageLabel> {
    preceded(
        tag("image_name="),
        tuple((alphanumericwithhypen, tag(".itb"))),
    )(input)
    .map(|(next_input, res)| (next_input, res))
}

fn image_version(input: &str) -> IResult<&str, u32> {
    preceded(
        tag("image_version="),
        separated_pair(tag("ver"), tag("_"), digit0),
    )(input)
    .map(|(next_input, res)| {
        (
            next_input,
            res.1.parse::<u32>().expect("not a valid version number"),
        )
    })
}

fn update_status(input: &str) -> IResult<&str, UpdateStatus> {
    alt((tag("Updating"), tag("Testing"), tag("Success")))(input)
        .map(|(next_input, res)| (next_input, res.into()))
}

fn ready_for_update(input: &str) -> IResult<&str, bool> {
    alt((tag("true"), tag("false")))(input).map(|(next_input, res)| {
        (
            next_input,
            bool::from_str(res).expect("not a boolean value"),
        )
    })
}

fn alphanumericwithhypen<T>(i: T) -> IResult<T, T>
where
    T: InputTakeAtPosition,
    <T as InputTakeAtPosition>::Item: AsChar,
{
    i.split_at_position1_complete(
        |item| {
            let char_item = item.as_char();
            !(char_item == '-') && !char_item.is_alphanum()
        },
        ErrorKind::AlphaNumeric,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::Error;

    #[test]
    fn test_config_keys() {
        assert_eq!(config_keys("[active]remaining "), Ok(("remaining ", ConfigKeys::Active)));
        // assert_eq!(config_keys("[passive]"), Ok(("", ConfigKeys::Passive)));
        // assert_eq!(
        //     config_keys("[passive]remaining"),
        //     Ok(("remaining", ConfigKeys::Passive))
        // );
        // assert_eq!(
        //     config_keys("prefix[passive]remaining"),
        //     Err(Err::Error(Error::new(
        //         "prefix[passive]remaining",
        //         ErrorKind::Tag
        //     )))
        // );
        // assert_eq!(
        //     config_keys("active"),
        //     Err(Err::Error(Error::new("active", ErrorKind::Tag)))
        // );
        // assert_eq!(
        //     config_keys("active]"),
        //     Err(Err::Error(Error::new("active]", ErrorKind::Tag)))
        // );
        // assert_eq!(
        //     config_keys("[]"),
        //     Err(Err::Error(Error::new("[]", ErrorKind::Tag)))
        // );
    }

    #[test]
    fn test_image_name() {
        assert_eq!(
            image_name("image_name=signed-apertis-rpi4.itbblah"),
            Ok(("blah", ("signed-apertis-rpi4", ".itb")))
        );
        assert_eq!(
            image_name("image_name=example.org:8080"),
            Err(Err::Error(Error::new(".org:8080", ErrorKind::Tag)))
        );
        assert_eq!(
            image_name("image_name=some-subsite.example.org:8080"),
            Err(Err::Error(Error::new(".example.org:8080", ErrorKind::Tag)))
        );
        assert_eq!(
            image_name("image_name=example.123"),
            Err(Err::Error(Error::new(".123", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_image_version() {
        assert_eq!(
            image_version("image_version=ver_612634867"),
            Ok(("", (612634867)))
        );
        assert_eq!(
            image_version("image_version=ver_111.222.345"),
            Err(Err::Error(Error::new("111.222.345", ErrorKind::Tag)))
        );
        // assert_eq!(
        //     image_name("image_name=some-subsite.example.org:8080"),
        //     Err(Err::Error(Error::new(".example.org:8080", ErrorKind::Tag)))
        // );
        // assert_eq!(
        //     image_name("image_name=example.123"),
        //     Err(Err::Error(Error::new(".123", ErrorKind::Tag)))
        // );
    }
}
