use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct GenericError(pub String);

impl Display for GenericError {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		f.write_str(&self.0)
	}
}

impl Error for GenericError {}

impl std::convert::From<std::option::NoneError> for GenericError {
	fn from(_err: std::option::NoneError) -> Self {
		GenericError("Value was None.".into())
	}
}

unsafe impl Send for GenericError {}
