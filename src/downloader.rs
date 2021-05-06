mod urltype;

use crate::converter::Converter;
use crate::error::{OkOrGeneric, Result};
use crate::requester::Requester;
use crate::util::remove_newline;
use rayon::prelude::*;
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
		&self,
		show_url: impl AsRef<str>,
		out_dir: impl Into<String>,
	) -> Result<()> {
		let show_url = show_url.as_ref();
		let out_dir = out_dir.into();
		println!("Downloading show...");
		let eps = self.requester.get_show_episodes(show_url).await?;
		let rt = tokio::runtime::Handle::current();
		eps.par_iter().for_each(|ep| {
			let result = rt.block_on(self.download_episode(&ep, &out_dir));
			match result {
				Ok(_) => println!("Download of {} succeeded.", ep),
				Err(_) => println!("Download of {} failed.", ep),
			}
		});
		Ok(())
	}

	async fn download_episode(&self, ep_url: &str, out_dir: &str) -> Result<()> {
		println!("Downloading episode {}", ep_url);
		let info = Requester::get_episode_info(ep_url).await?;
		let url = self.requester.get_media_url(info.id).await?;
		let content = Self::get_as_string(&url).await?;
		let mut path = path::PathBuf::from(out_dir);
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

	pub async fn download(&self, out_dir: impl AsRef<str>, url: impl AsRef<str>) -> Result<()> {
		let out_dir = out_dir.as_ref();
		let url = Self::sanitize_url(url.as_ref()).await;
		Downloader::verify_url(url).await?;
		let url_type = URLType::get(url)?;
		match url_type {
			URLType::Playlist => self.download_show(url, out_dir).await,
			URLType::Video => self.download_episode(url, out_dir).await,
		}
	}
}
