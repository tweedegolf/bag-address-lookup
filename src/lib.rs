mod database;
mod service;

#[cfg(feature = "create")]
mod logging;

#[cfg(feature = "create")]
mod transform;

#[cfg(feature = "create")]
mod create;

#[cfg(feature = "create")]
mod parsing;

pub use database::{Database, DatabaseError, DatabaseHandle, NumberRange, encode_pc};
pub use service::{serve, serve_with_shutdown};

#[cfg(feature = "create")]
pub use logging::log_with_elapsed;

#[cfg(feature = "create")]
pub use create::create_database;

#[cfg(feature = "create")]
pub use parsing::{Address, Locality, PublicSpace};

#[cfg(feature = "create")]
pub use transform::{LocalityMap, encode_addresses, index_localities, index_public_spaces};
