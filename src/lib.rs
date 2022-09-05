#![no_std]

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit0, multispace0},
    combinator::opt,
    error::ErrorKind,
    sequence::{preceded, separated_pair, tuple},
    AsChar, IResult, InputTakeAtPosition,
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

/// A label consists of a `filename` and a file extension (ex: `.itb`)
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
            "updating" => UpdateStatus::Updating,
            "testing" => UpdateStatus::Testing,
            "success" => UpdateStatus::Success,
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
    preceded(
        tag("update_status="),
        alt((tag("updating"), tag("testing"), tag("success"))),
    )(input)
    .map(|(next_input, res)| (next_input, res.into()))
}

fn ready_for_update(input: &str) -> IResult<&str, bool> {
    preceded(
        tag("ready_for_update_flag="),
        alt((tag("true"), tag("false"))),
    )(input)
    .map(|(next_input, res)| {
        (
            next_input,
            bool::from_str(res).expect("not a boolean value"),
        )
    })
}

fn active_config(input: &str) -> IResult<&str, ActiveConf> {
    tuple((
        multispace0,
        config_keys,
        multispace0,
        image_name,
        multispace0,
        image_version,
        multispace0,
    ))(input)
    .map(|(next_input, res)| {
        let (_crlf0, active_config, _crlf1, image_name, _crlf2, image_version, _crlf3) = res;
        (
            next_input,
            ActiveConf {
                active_config,
                image_name,
                image_version,
            },
        )
    })
}

fn passive_config(input: &str) -> IResult<&str, PassiveConf> {
    tuple((
        multispace0,
        config_keys,
        multispace0,
        ready_for_update,
        multispace0,
        opt(image_name),
        multispace0,
        opt(image_version),
        multispace0,
        opt(update_status),
        multispace0,
    ))(input)
    .map(|(next_input, res)| {
        let (
            _crlf0,
            passive_config,
            _crlf1,
            ready_for_update_flag,
            _crlf2,
            image_name,
            _crlf3,
            image_version,
            _crlf4,
            update_status,
            _crlf5,
        ) = res;
        (
            next_input,
            PassiveConf {
                passive_config,
                ready_for_update_flag,
                image_name,
                image_version,
                update_status,
            },
        )
    })
}

pub fn parse_config(input: &str) -> IResult<&str, (ActiveConf, PassiveConf)> {
    tuple((active_config, passive_config))(input)
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
    // use libc_print::libc_println;
    use nom::{error::Error, Err};

    #[test]
    fn test_config_keys() {
        assert_eq!(config_keys("[active]"), Ok(("", ConfigKeys::Active)));
        assert_eq!(config_keys("[passive]"), Ok(("", ConfigKeys::Passive)));
        assert_eq!(
            config_keys("[passive]remaining"),
            Ok(("remaining", ConfigKeys::Passive))
        );
        assert_eq!(
            config_keys("prefix[passive]remaining"),
            Err(Err::Error(Error::new(
                "prefix[passive]remaining",
                ErrorKind::Tag
            )))
        );
        assert_eq!(
            config_keys("active"),
            Err(Err::Error(Error::new("active", ErrorKind::Tag)))
        );
        assert_eq!(
            config_keys("active]"),
            Err(Err::Error(Error::new("active]", ErrorKind::Tag)))
        );
        assert_eq!(
            config_keys("[]"),
            Err(Err::Error(Error::new("[]", ErrorKind::Tag)))
        );
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
    }

    #[test]
    fn test_ready_for_update() {
        assert_eq!(
            ready_for_update("ready_for_update_flag=true"),
            Ok(("", true))
        );
    }

    #[test]
    fn test_update_status() {
        // libc_println!(
        //     "active_config: {:?}",
        //     update_status("update_status=updating")
        // );
        assert_eq!(
            update_status("update_status=updating"),
            Ok(("", UpdateStatus::Updating))
        );
    }

    #[test]
    fn test_active_conf() {
        assert_eq!(
            active_config(
                "
                [active]
            image_name=xx.itb
            image_version=ver_123 "
            ),
            Ok((
                "",
                ActiveConf {
                    active_config: ConfigKeys::Active,
                    image_name: ("xx", ".itb"),
                    image_version: 123
                }
            ))
        );
    }

    #[test]
    fn test_passive_conf() {
        assert_eq!(
            passive_config(
                "
                [passive]
                ready_for_update_flag=true
                image_name=xx.itb
                image_version=ver_123
                update_status=updating "
            ),
            Ok((
                "",
                PassiveConf {
                    passive_config: ConfigKeys::Passive,
                    ready_for_update_flag: true,
                    image_name: Some(("xx", ".itb")),
                    image_version: Some(123),
                    update_status: Some(UpdateStatus::Updating)
                }
            ))
        );
        assert_eq!(
            passive_config(
                "
                [passive]
                ready_for_update_flag=false
                image_name=
                image_version=none
                update_status=none "
            ),
            Ok((
                "image_name=
                image_version=none
                update_status=none ",
                PassiveConf {
                    passive_config: ConfigKeys::Passive,
                    ready_for_update_flag: false,
                    image_name: None,
                    image_version: None,
                    update_status: None
                }
            ))
        );
    }

    #[test]
    fn test_parse_config() {
        assert_eq!(
            parse_config(
                "[active]
                image_name=xx.itb
                image_version=ver_34488734
                
                [passive]
                ready_for_update_flag=true
                image_name=xx.itb
                image_version=ver_34488735
                update_status=updating"
            ),
            Ok((
                "",
                (
                    ActiveConf {
                        active_config: ConfigKeys::Active,
                        image_name: ("xx", ".itb"),
                        image_version: 34488734
                    },
                    PassiveConf {
                        passive_config: ConfigKeys::Passive,
                        ready_for_update_flag: true,
                        image_name: Some(("xx", ".itb")),
                        image_version: Some(34488735),
                        update_status: Some(UpdateStatus::Updating)
                    }
                )
            ))
        );
        assert_eq!(
            parse_config(
                "[active]
                image_name=xx.itb
                image_version=ver_34488734
                
                [passive]
                ready_for_update_flag=false
                image_name=
                image_version=ver_34488735
                update_status=updating"
            ),
            Ok((
                "image_name=
                image_version=ver_34488735
                update_status=updating",
                (
                    ActiveConf {
                        active_config: ConfigKeys::Active,
                        image_name: ("xx", ".itb"),
                        image_version: 34488734
                    },
                    PassiveConf {
                        passive_config: ConfigKeys::Passive,
                        ready_for_update_flag: false,
                        image_name: None,
                        image_version: None,
                        update_status: None
                    }
                )
            ))
        );
    }
}
