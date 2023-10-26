pub mod generic_error;
pub mod ok_or_generic;

pub use generic_error::GenericError;
pub use ok_or_generic::OkOrGeneric;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
