mod urltype;

use crate::converter::Converter;
use crate::error::{OkOrGeneric, Result};
use crate::event_subscriber::EventSubscriber;
use crate::requester::{EpisodeInfo, Requester};
use crate::util::remove_newline;
use rayon::prelude::*;
use reqwest::StatusCode;
use std::path;
use urltype::URLType;

lazy_static! {
	static ref DR_EP_URL_REGEX: regex::Regex =
		regex::Regex::new(r#"(((https)|(http))(://www\.dr\.dk/drtv/)).*_\d+"#).unwrap();
}

#[derive(Clone)]
pub struct EpisodeData {
	info: EpisodeInfo,
	data: Vec<u8>,
}

pub type EpisodeCollection = Vec<Option<EpisodeData>>;

pub struct Downloader<'a> {
	requester: Requester,
	converter: Option<Converter>,
	subscriber: Option<EventSubscriber<'a>>,
}

impl<'a> Downloader<'a> {
	/// Create a new Downloader.
	pub fn new(requester: Requester) -> Self {
		Downloader {
			requester,
			converter: None,
			subscriber: None,
		}
	}

	pub fn with_converter(mut self, converter: Converter) -> Self {
		self.converter = Some(converter);
		self
	}

	// Add an EventSubscriber.
	pub fn with_subscriber(mut self, subscriber: EventSubscriber<'a>) -> Self {
		self.subscriber = Some(subscriber);
		self
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

	async fn download_episode_raw<T: Into<String>>(&self, ep_url: T) -> Result<EpisodeData> {
		let ep_url = ep_url.into();
		if let Some(sub) = &self.subscriber {
			sub.on_download(&ep_url);
		}
		let info = Requester::get_episode_info(ep_url).await?;
		let url = self.requester.get_episode_url(&info.id).await?;
		let content = Self::get_as_string(&url).await?;
		Ok(EpisodeData {
			info,
			data: content.as_bytes().into(),
		})
	}

	async fn download_show_raw(&self, show_url: &str) -> Result<EpisodeCollection> {
		if let Some(sub) = &self.subscriber {
			sub.on_download(show_url);
		}
		let eps = self.requester.get_show_episodes(show_url).await?;
		let rt = tokio::runtime::Handle::current();
		let show_data = eps
			.into_par_iter()
			.map(|ep| {
				let url_copy = ep.clone();
				let result = rt.block_on(self.download_episode_raw(ep));
				match result {
					Ok(data) => {
						if let Some(sub) = &self.subscriber {
							sub.on_finish(&url_copy);
						}
						Some(data)
					}
					Err(_) => {
						if let Some(sub) = &self.subscriber {
							sub.on_failed(&url_copy);
						}
						None
					}
				}
			})
			.collect::<EpisodeCollection>();
		Ok(show_data)
	}

	async fn download_show(
		&self,
		show_url: impl AsRef<str>,
		out_dir: impl Into<String>,
	) -> Result<()> {
		let show_url = show_url.as_ref();
		let out_dir = out_dir.into();
		for ep in self.download_show_raw(show_url).await?.iter().flatten() {
			let mut path = path::PathBuf::from(&out_dir);
			path.push(format!("./{}.mp4", ep.info.name));
			if let Some(sub) = &self.subscriber {
				sub.on_convert(&ep.info.name);
			}
			if let Some(con) = &self.converter {
				con.convert(&ep.data, path.to_str().ok_or_generic("Path was invalid.")?)?;
			}
		}
		Ok(())
	}

	async fn download_episode<T: Into<String>>(&self, ep_url: T, out_dir: &str) -> Result<()> {
		let data = self.download_episode_raw(ep_url).await?;
		let mut path = path::PathBuf::from(out_dir);
		path.push(format!("./{}.mp4", data.info.name));
		if let Some(con) = &self.converter {
			con.convert(
				&data.data,
				path.to_str().ok_or_generic("Path was invalid.")?,
			)?;
		}
		Ok(())
	}

	fn sanitize_url(mut url: &str) -> &str {
		url = remove_newline(url);
		url
	}

	/// Download media from url to a Vec of optional EpisodeData.
	pub async fn download_raw(&self, url: impl AsRef<str>) -> Result<EpisodeCollection> {
		let url = Self::sanitize_url(url.as_ref());
		Downloader::verify_url(url).await?;
		let url_type = URLType::get(url)?;
		match url_type {
			URLType::Playlist => Ok::<EpisodeCollection, Box<dyn std::error::Error>>(
				self.download_show_raw(url).await?,
			),
			URLType::Video => Ok::<EpisodeCollection, Box<dyn std::error::Error>>(vec![Some(
				self.download_episode_raw(url).await?,
			)]),
		}
	}

	/// Download media from url to the specified path.
	pub async fn download(&self, out_dir: impl AsRef<str>, url: impl AsRef<str>) -> Result<()> {
		let out_dir = out_dir.as_ref();
		let url = Self::sanitize_url(url.as_ref());
		Downloader::verify_url(url).await?;
		let url_type = URLType::get(url)?;
		match url_type {
			URLType::Playlist => self.download_show(url, out_dir).await,
			URLType::Video => self.download_episode(url, out_dir).await,
		}
	}
}
