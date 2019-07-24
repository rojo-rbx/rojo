mod interface;
mod legacy;
mod real_fetcher;

pub use legacy::*;

pub mod new {
    pub use super::interface::*;
    pub use super::real_fetcher::*;
}