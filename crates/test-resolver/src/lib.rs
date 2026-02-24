pub mod errors;
pub mod injection;
pub mod loader;
pub mod metadata;
pub mod trace;

pub use injection::*;
pub use loader::*;
pub use metadata::*;
pub use trace::*;

pub fn resolver_name() -> &'static str {
    "test-resolver"
}
