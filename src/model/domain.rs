use std::str::FromStr;

use failure::Fail;

#[derive(Clone, Debug, Fail)]
#[fail(display = "Invalid domain string '{}'", _0)]
pub struct DomainParseError(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Domain {
    Japan,
    NorthAmerica,
    Europe,
    France,
    Germany,
}

impl Domain {
    pub fn to_string(&self) -> &str {
        match self {
            Domain::Japan => "jp",
            Domain::NorthAmerica => "na",
            Domain::Europe => "eu",
            Domain::France => "fr",
            Domain::Germany => "de",
        }
    }
}

impl FromStr for Domain {
    type Err = DomainParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jp" => Ok(Domain::Japan),
            "na" => Ok(Domain::NorthAmerica),
            "eu" => Ok(Domain::Europe),
            "fr" => Ok(Domain::France),
            "de" => Ok(Domain::Germany),
            x => Err(DomainParseError(x.into())),
        }
    }
}
