use crate::converter::Converter;
use crate::error::GenericError;
use crate::requester::{Requester, Result};
use reqwest::StatusCode;

lazy_static! {
	static ref DR_VID_URL_REGEX: regex::Regex =
		regex::Regex::new(r#"((https)|(http))(://www.dr.dk/drtv/se/).+(\d)"#).unwrap();
}

pub struct Downloader {
	requester: Requester,
	converter: Converter,
}

impl Downloader {
	pub fn new(requester: Requester, converter: Converter) -> Self {
		Downloader {
			requester,
			converter,
		}
	}
}

impl Downloader {
	async fn verify_url(url: &str) -> Result<'_, ()> {
		if !DR_VID_URL_REGEX.is_match(url) {
			return Err("Unrecognzed URL.".into());
		}
		Ok(())
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

	pub async fn download(
		&mut self,
		path: impl AsRef<str>,
		video_url: impl AsRef<str>,
	) -> Result<'_, ()> {
		Downloader::verify_url(video_url.as_ref()).await?;
		println!("Starting download...");
		let id = Requester::get_video_id(video_url.as_ref()).await?;
		let url = self.requester.get_media_url(id).await?;
		let content = Self::get_as_string(&url).await?;
		self.converter.convert(content.as_bytes(), path)?;
		Ok(())
	}
}
