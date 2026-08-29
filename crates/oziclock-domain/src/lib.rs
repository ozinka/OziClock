//! Framework-free domain types and rules for OziClock.

mod clock;

pub use clock::{Clock, ClockCollection};

/// Product name shared by all front ends.
pub const PRODUCT_NAME: &str = "OziClock";
