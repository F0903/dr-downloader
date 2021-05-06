mod urltype;

use crate::converter::Converter;
use crate::error::{OkOrGeneric, Result};
use crate::requester::Requester;
use crate::util::remove_newline;
use reqwest::StatusCode;
use std::path;
use urltype::URLType;

lazy_static! {
	static ref DR_EP_URL_REGEX: regex::Regex =
		regex::Regex::new(r#"(((https)|(http))(://www\.dr\.dk/drtv/)).*_\d+"#).unwrap();
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

	async fn verify_url(url: &str) -> Result<()> {
		if !DR_EP_URL_REGEX.is_match(url) {
			return Err("Unrecognzed URL.".into());
		}
		Ok(())
	}

	async fn get_as_string(url: &str) -> Result<String> {
		let result = reqwest::get(url).await?;
		let status = result.status();
		if status != StatusCode::OK {
			return Err(format!("Status code was not 200 OK.\nCode: {}", status).into());
		}
		let text = result.text().await?;
		Ok(text)
	}

	async fn download_show(
		&'static self,
		show_url: impl AsRef<str>,
		out_dir: impl ToString,
	) -> Result<()> {
		println!("Downloading show...");
		let eps = self.requester.get_show_episodes(show_url.as_ref()).await?;
		let mut tasks = vec![];
		for ep in eps {
			let dir = out_dir.to_string();
			tasks.push(tokio::spawn(async move {
				let result = self.download_episode(&ep, dir).await;
				match result {
					Ok(_) => println!("Download of {} succeeded.", ep),
					Err(_) => println!("Download of {} failed.", ep),
				}
			}));
		}
		futures::future::join_all(tasks).await;
		Ok(())
	}

	async fn download_episode(
		&self,
		ep_url: impl AsRef<str>,
		out_dir: impl AsRef<str>,
	) -> Result<()> {
		println!("Downloading episode {}", ep_url.as_ref());
		let info = Requester::get_episode_info(ep_url.as_ref()).await?;
		let url = self.requester.get_media_url(info.id).await?;
		let content = Self::get_as_string(&url).await?;
		let mut path = path::PathBuf::from(out_dir.as_ref());
		path.push(format!("./{}.mp4", info.name));
		self.converter.convert(
			content.as_bytes(),
			path.to_str().ok_or_generic("Path was invalid.")?,
		)?;
		Ok(())
	}

	async fn sanitize_url(mut url: &str) -> &str {
		url = remove_newline(url);
		url
	}

	pub async fn download(
		&'static self,
		out_dir: impl AsRef<str>,
		url: impl AsRef<str>,
	) -> Result<()> {
		let url = Self::sanitize_url(url.as_ref()).await;
		Downloader::verify_url(url).await?;
		let url_type = URLType::get(url)?;
		match url_type {
			URLType::Playlist => self.download_show(url, out_dir.as_ref()).await,
			URLType::Video => self.download_episode(url, out_dir).await,
		}
	}
}
