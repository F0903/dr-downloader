use std::convert::From;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::option::NoneError;

#[derive(Debug)]
pub struct GenericError(pub String);

impl Display for GenericError {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		f.write_str(&self.0)
	}
}

impl From<NoneError> for GenericError {
	fn from(_err: NoneError) -> Self {
		GenericError(String::from("Option contained no values."))
	}
}

impl Error for GenericError {}

impl From<&str> for GenericError {
	fn from(val: &str) -> Self {
		GenericError(val.to_owned())
	}
}

impl From<String> for GenericError {
	fn from(val: String) -> Self {
		GenericError(val)
	}
}

impl From<std::io::Error> for GenericError {
	fn from(err: std::io::Error) -> Self {
		GenericError(err.to_string())
	}
}
