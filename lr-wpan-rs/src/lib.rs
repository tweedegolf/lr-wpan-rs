#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![allow(async_fn_in_trait)]

extern crate alloc;
extern crate core;

use crate::wire::{ExtendedAddress, ShortAddress};

// This must go FIRST so that all the other modules see its macros.
mod fmt;

pub mod consts;
pub mod mac;
pub mod phy;
pub mod pib;
mod reqresp;
pub mod sap;
pub mod time;
pub mod wire;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceAddress {
    Short(ShortAddress),
    Extended(ExtendedAddress),
}

/// The existing channel pages as defined in 8.1.2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ChannelPage {
    #[default]
    Mhz868_915_2450 = 0,
    Mhz868_915_1 = 1,
    Mhz868_915_2 = 2,
    Css = 3,
    Uwb = 4,
    Mhz780 = 5,
    Mhz950 = 6,
}

impl TryFrom<u8> for ChannelPage {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Mhz868_915_2450),
            1 => Ok(Self::Mhz868_915_1),
            2 => Ok(Self::Mhz868_915_2),
            3 => Ok(Self::Css),
            4 => Ok(Self::Uwb),
            5 => Ok(Self::Mhz780),
            6 => Ok(Self::Mhz950),
            _ => Err(value),
        }
    }
}

impl ChannelPage {
    /// Get the initial contention window length CW0 for the page.
    /// Defined in 5.1.1.4
    pub fn cw0(&self) -> u8 {
        match self {
            ChannelPage::Mhz868_915_2450 => 2,
            ChannelPage::Mhz868_915_1 => 2,
            ChannelPage::Mhz868_915_2 => 2,
            ChannelPage::Css => 2,
            ChannelPage::Uwb => 2,
            ChannelPage::Mhz780 => 2,
            ChannelPage::Mhz950 => 1,
        }
    }
}
