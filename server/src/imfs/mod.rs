mod interface;
mod legacy;

pub use legacy::*;

pub mod new {
    pub use super::interface::*;
}