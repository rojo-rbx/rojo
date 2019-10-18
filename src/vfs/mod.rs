mod error;
mod event;
mod fetcher;
mod noop_fetcher;
mod real_fetcher;
mod snapshot;

// I don't think module inception is a real problem?
#[allow(clippy::module_inception)]
mod vfs;

pub use error::*;
pub use event::*;
pub use fetcher::*;
pub use noop_fetcher::*;
pub use real_fetcher::*;
pub use snapshot::*;
pub use vfs::*;

#[cfg(test)]
mod test_fetcher;

#[cfg(test)]
pub use test_fetcher::*;
