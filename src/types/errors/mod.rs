mod config;
mod filter;
mod main;
mod order;
mod risk;

pub use config::ConfigError;
pub use filter::FilterError;
pub use main::Error;
pub use order::OrderError;
pub use risk::RiskError;

/// This is defined as a convenience.
pub type Result<Inner, I, const D: u8> = std::result::Result<Inner, Error<I, D>>;
