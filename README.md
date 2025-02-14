# lr-wpan-rs

> *low-rate wireless personal area network in Rust*

[![crates.io](https://img.shields.io/crates/v/lr-wpan-rs.svg)](https://crates.io/crates/lr-wpan-rs) [![Documentation](https://docs.rs/lr-wpan-rs/badge.svg)](https://docs.rs/lr-wpan-rs)

Rust implementation for the IEEE 802.15.4 protocol.

We're just setting this up as an open source project and we're looking funding to work on this implementation.  
If you want to see this implementation finished like us, contact us: dion@tweedegolf.com.

## Goals

- Provide a full IEEE 802.15.4-2011 implementation, using the protocol-defined interface layers
- Eventually maybe upgrade to the most recent spec (which can do more, but is also more complex)
- Be hardware agnostic, so it can run on any radio that implements the phy and can run on any microcontroller that's capable enough
- Use async to its fullest extend, 'real-time' scheduling is left to the radio driver
- Everything fully in stable Rust, preferrably without using unsafe

## Current state

- Not enough is implemented to be useful to anyone yet
- More cleanup and review is needed. Initial development was focussed on getting an MVP out of the door.

Initial development paid for by [Rocsys](https://www.rocsys.com/).
