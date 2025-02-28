# lr-wpan-rs

> *low-rate wireless personal area network in Rust*

[![crates.io](https://img.shields.io/crates/v/lr-wpan-rs.svg)](https://crates.io/crates/lr-wpan-rs) [![Documentation](https://docs.rs/lr-wpan-rs/badge.svg)](https://docs.rs/lr-wpan-rs)

Rust implementation for the IEEE 802.15.4 protocol.

We're just setting this up as an open source project and we're looking for funding to work on this implementation.  
If you want to see this implementation finished like us or know someone who does, contact us: dion@tweedegolf.com.

Initial development paid for by [Rocsys](https://www.rocsys.com/).

## Goals

- Provide a full IEEE 802.15.4-2011 implementation, using the protocol-defined interface layers
- Eventually maybe upgrade to the most recent spec (which can do more, but is also more complex)
- Follow the spec relatively closely
- Be hardware agnostic, so it can run on any radio that implements the phy and can run on any microcontroller that's capable enough
- Use async to its fullest extend, 'real-time' scheduling is left to the radio driver
- Everything fully in stable Rust
- Fully no-std, and no-alloc

## Current state

There's lots there already, but not enough is implemented to be useful to anyone yet.
Rows with a bullet in the MVP column are required for a reasonable minimum implementation.
The list is probably not exhaustive.

| Status | Feature                 |  MVP  | Notes                                                             | Effort estimation*                       |
| :----: | :---------------------- | :---: | :---------------------------------------------------------------- | :--------------------------------------- |
|  ✅/🚧   | Phy trait               |   ⦿   | Radio abstraction in good shape, but might need some more changes |                                          |
|   ✅    | Phy PIB                 |   ⦿   |                                                                   |                                          |
|   ✅    | Mac PIB                 |   ⦿   |                                                                   |                                          |
|   ✅    | SAP message definitions |   ⦿   |                                                                   |                                          |
|   ✅    | MLME reset              |   ⦿   |                                                                   |                                          |
|   ✅    | MLME set                |   ⦿   |                                                                   |                                          |
|   ✅    | MLME get                |   ⦿   |                                                                   |                                          |
|   ✅    | MLME start              |   ⦿   |                                                                   |                                          |
|   🚧    | MLME scan               |   ⦿   | Active and passive implemented, ED and orphan scans still todo    | 3 days                                   |
|   🚧    | Data request support    |   ⦿   |                                                                   | 3 days                                   |
|   🚧    | Acknowledgement support |   ⦿   | Including retransmissions                                         | 2 days                                   |
|   🚧    | MLME associate          |   ⦿   |                                                                   | 2 days                                   |
|   ❌    | MLME disassociate       |   ⦿   |                                                                   | 4 days                                   |
|   ❌    | MLME poll               |   ⦿   | Builds on the 'data request support'                              | 1 day                                    |
|   ❌    | MCPS data               |   ⦿   | Builds on the 'data request support'                              | 2 days                                   |
|   ❌    | MLME sync               |       |                                                                   | 6 days                                   |
|   ❌    | PAN id conflict         |       |                                                                   | 2 days                                   |
|   ❌    | MLME orphan             |       | Including coordinator realignment, builds on 'PAN id conflict'    | 5 days                                   |
|   ❌    | MLME gts                |       | Including full gts handling                                       | 4 days                                   |
|   ❌    | MLME beacon notify      |       |                                                                   | 2 days                                   |
|   ❌    | MLME comm status        |       |                                                                   | 1 day                                    |
|   ❌    | MCPS purge              |       |                                                                   | 1 day                                    |
|   ❌    | MLME dps                |       |                                                                   | 2 days                                   |
|   ❌    | MLME calibrate          |       |                                                                   | 1 day                                    |
|   ❌    | MLME sounding           |       |                                                                   | 1 day                                    |
|   ❌    | Promiscuous mode        |       |                                                                   | 1 day                                    |
|  🚧/❌   | Frame security          |       | Parts implemented, but not enabled. Unclear how much work is left | ? days                                   |
|   🚧    | Testing                 |       | Lots being tested, but can be structured better                   | 10 days (spread over all other features) |

_* The estimation is rough. Past experience has tought us that often things turn out to be more complex than they seem at first._
_If that keeps happening for everything, the real work could be 2x the estimation._
