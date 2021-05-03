use crate::converter::Converter;
use crate::error::GenericError;
use crate::requester::{Requester, Result};
use reqwest::StatusCode;
use std::path;

lazy_static! {
	static ref DR_VID_URL_REGEX: regex::Regex =
		regex::Regex::new(r#"(((https)|(http))(://www\.dr\.dk/drtv/)).*_\d+"#).unwrap();
}

enum URLType {
	Video,
	Playlist,
}

impl URLType {
	pub fn get(url: &str) -> Result<'_, URLType> {
		if url.contains("saeson") {
			return Ok(URLType::Playlist);
		} else if url.contains("episode") || url.contains("se") {
			return Ok(URLType::Video);
		}
		Err("Could not parse URL Type.".into())
	}
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

	async fn download_playlist(&mut self, playlist_url: &str, out_dir: &str) -> Result<'_, ()> {
		println!("Downloading playlist...");
		let eps = self.requester.get_playlist_videos(playlist_url).await?;
		for ep in eps {
			let result = self.download_video(&ep, out_dir).await;
			if result.is_err() {
				const DELAY: u16 = 3000;
				println!(
					"Something went wrong with last download. Continuing to next video in {}s...",
					DELAY / 1000
				);
				std::thread::sleep(std::time::Duration::from_millis(DELAY as u64));
			}
		}
		Ok(())
	}

	async fn download_video(&mut self, video_url: &str, out_dir: &str) -> Result<'_, ()> {
		println!("Downloading video...");
		let info = Requester::get_video_info(video_url).await?;
		let url = self.requester.get_media_url(info.id).await?;
		let content = Self::get_as_string(&url).await?;
		let mut path = path::PathBuf::from(out_dir);
		path.push(format!("./{}.mp4", info.name));
		self.converter.convert(
			content.as_bytes(),
			path.to_str()
				.ok_or_else(|| GenericError("Path was invalid.".into()))?,
		)?;
		Ok(())
	}

	pub async fn download(
		&mut self,
		out_dir: impl AsRef<str>,
		url: impl AsRef<str>,
	) -> Result<'_, ()> {
		Downloader::verify_url(url.as_ref()).await?;
		let url_type = URLType::get(url.as_ref())?;
		match url_type {
			URLType::Playlist => self.download_playlist(url.as_ref(), out_dir.as_ref()).await,
			URLType::Video => self.download_video(url.as_ref(), out_dir.as_ref()).await,
		}
	}
}
