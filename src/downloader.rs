use crate::error::GenericError;
use crate::requester::{Requester, Result};
use reqwest::StatusCode;
use std::fs;

pub struct Downloader {
	requester: Requester,
}

impl Downloader {
	pub fn new(requester: Requester) -> Self {
		Downloader { requester }
	}

	async fn get_as_string(url: &str) -> Result<'_, String> {
		let result = reqwest::get(url).await?;
		let status = result.status();
		if status != StatusCode::OK {
			return Err(
				GenericError(format!("Status code was not 200 OK.\nCode: {}", status)).into(),
			);
		}
		let text = result.text().await?;
		Ok(text)
	}

	pub async fn download<T: AsRef<str>>(&mut self, path: T, video_url: &str) -> Result<'_, ()> {
		let id = Requester::get_video_id(video_url).await?;
		let url = self.requester.get_media_url(id).await?;
		let content = Self::get_as_string(&url).await?;
		fs::write(path.as_ref(), content)?;
		Ok(())
	}
}
