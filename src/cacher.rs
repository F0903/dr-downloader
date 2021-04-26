use std::fs::{read, write};
use std::io::{Error, ErrorKind, Result};

const TOKEN_PATH: &str = "./token.txt";

pub fn get_token() -> Result<String> {
	let token = read(TOKEN_PATH)?;
	String::from_utf8(token).map_err(|_e| Error::from(ErrorKind::InvalidData))
}

pub fn cache_token<T: AsRef<str>>(token: T) -> Result<()> {
	write(TOKEN_PATH, token.as_ref())
}

pub async fn get_or_cache_token<
	E: std::error::Error + ?Sized,
	T: std::future::Future<Output = std::result::Result<String, Box<E>>>,
	F: Fn() -> T,
>(
	token_factory: F,
) -> Result<String> {
	if let Ok(token) = get_token() {
		return Ok(token);
	}
	let token = token_factory().await.map_err(|_e| ErrorKind::Other)?;
	cache_token(&token).ok();
	Ok(token)
}
