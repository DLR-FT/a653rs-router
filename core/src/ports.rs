use crate::error::Error;
use std::str::{FromStr, Utf8Error};

use apex_rs::{
    bindings::ApexName,
    prelude::{
        ApexSamplingPortP1, ApexSamplingPortP4, ApexUnsigned, MessageSize, Name,
        SamplingPortDestination, SamplingPortSource,
    },
};

// TODO link -> channel name -> port config -> port destination / source

pub struct PortLookupError(Error);

impl From<Error> for PortLookupError {
    fn from(val: Error) -> Self {
        PortLookupError(val)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub struct VirtualLinkId(u16);

impl VirtualLinkId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn into_inner(self) -> u16 {
        self.0
    }

    pub fn as_u8(self) -> u8 {
        let v = self.0 as u8;
        v
    }

    pub fn as_u16(self) -> u16 {
        self.into_inner()
    }
}

impl Default for VirtualLinkId {
    fn default() -> Self {
        VirtualLinkId(0)
    }
}

struct VirtualLink;

pub trait LinkLookup {
    fn get_destination(source: VirtualLinkId) -> Result<VirtualLink, Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelName(Name);

impl ChannelName {
    pub fn new(name: Name) -> Self {
        ChannelName(name)
    }

    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        self.0.to_str()
    }

    pub fn into_inner(self) -> Name {
        self.0
    }
}

impl Default for ChannelName {
    fn default() -> Self {
        let invalid = ApexName::default();
        ChannelName(Name::new(invalid))
    }
}

impl FromStr for ChannelName {
    type Err = ApexUnsigned;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = Name::from_str(s)?;
        Ok(Self(name))
    }
}

impl From<ChannelName> for Name {
    fn from(val: ChannelName) -> Self {
        val.0
    }
}

impl From<Name> for ChannelName {
    fn from(val: Name) -> Self {
        ChannelName(val)
    }
}
