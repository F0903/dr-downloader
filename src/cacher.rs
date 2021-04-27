use std::io::{ErrorKind, Result};
use winreg::{enums, HKEY};

//TODO: Make a cacher for linux or mac

const BASE_PATH: HKEY = enums::HKEY_CURRENT_USER;
const KEY_PATH: &str = "SOFTWARE\\dr-downloader";

pub fn get_token() -> Result<String> {
	let key = winreg::RegKey::predef(BASE_PATH).open_subkey(KEY_PATH)?;
	key.get_value("token")
}

pub fn cache_token<T: AsRef<str>>(token: T) -> Result<()> {
	let (key, _disp) = winreg::RegKey::predef(BASE_PATH).create_subkey(KEY_PATH)?;
	key.set_value("token", &token.as_ref())
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
	cache_token(&token)?;
	Ok(token)
}
