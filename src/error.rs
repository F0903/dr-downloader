#[derive(Debug)]
pub struct GenericError(pub String);

impl std::fmt::Display for GenericError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for GenericError {}

impl std::convert::From<std::option::NoneError> for GenericError {
	fn from(_err: std::option::NoneError) -> Self {
		GenericError("Value was None.".into())
	}
}
