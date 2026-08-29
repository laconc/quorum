//! Test infrastructure shared across the workspace.
//!
//! The only thing here today is [`Clock`], because time is the one ambient
//! dependency the whole system needs and the one that quietly destroys
//! testability if it is read directly. Cure periods, term expiries, vote
//! closings, delay windows, and suspension windows are all defined by elapsed
//! time; none of them can be tested if the current instant is read from the
//! operating system at the point of use.
//!
//! Every crate takes a `&dyn Clock` (or a generic `C: Clock`) rather than
//! calling the standard library. A lint forbids the alternative: see
//! `app/clippy.toml`.

pub mod clock;

pub use clock::{AdvancingClock, Clock, FixedClock, SystemClock};
