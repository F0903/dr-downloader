use crate::error::Result;

pub enum URLType {
	Video,
	Playlist,
}

impl URLType {
	pub fn get(url: &str) -> Result<URLType> {
		if url.contains("saeson") || url.contains("serie") {
			return Ok(URLType::Playlist);
		} else if url.contains("episode") || url.contains("se") {
			return Ok(URLType::Video);
		}
		Err("Could not parse URL Type.".into())
	}
}
