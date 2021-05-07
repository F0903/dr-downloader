use super::generic_error::GenericError;

pub trait OkOrGeneric<T> {
    fn ok_or_generic<S: ToString>(self, msg: S) -> Result<T, GenericError>;
}

impl<T> OkOrGeneric<T> for Option<T> {
    fn ok_or_generic<S: ToString>(self, msg: S) -> Result<T, GenericError> {
        self.ok_or_else(|| GenericError(msg.to_string()))
    }
}
