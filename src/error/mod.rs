pub mod generic_error;
pub mod ok_or_generic;

pub use generic_error::GenericError;
pub use ok_or_generic::OkOrGeneric;

type ErrorType = Box<dyn std::error::Error>;
pub type Result<T, E = ErrorType> = std::result::Result<T, E>;
