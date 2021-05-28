use crate::error::Result;
use crate::event_subscriber::EventSubscriber;
use crate::models::{episode::EpisodeData, URLType};
use crate::requester::Requester;
use crate::util::remove_newline;
use rayon::prelude::*;
use reqwest::StatusCode;

lazy_static! {
	static ref DR_EP_URL_REGEX: regex::Regex =
		regex::Regex::new(r#"(((https)|(http))(://www\.dr\.dk/drtv/)).*_\d+"#).unwrap();
}

pub type EpisodeCollection = Vec<Option<EpisodeData>>;

pub struct Downloader<'a> {
	requester: Requester,
	subscriber: Option<EventSubscriber<'a>>,
}

impl<'a> Downloader<'a> {
	/// Create a new Downloader.
	pub fn new(requester: Requester) -> Self {
		Downloader {
			requester,
			subscriber: None,
		}
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

	pub(crate) async fn download_episode<T: Into<String>>(&self, ep_url: T) -> Result<EpisodeData> {
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

	pub(crate) async fn download_show(&self, show_url: &str) -> Result<EpisodeCollection> {
		if let Some(sub) = &self.subscriber {
			sub.on_download(show_url);
		}
		let eps = self.requester.get_show_episodes(show_url).await?;
		let rt = tokio::runtime::Handle::current();
		let show_data = eps
			.into_par_iter()
			.map(|ep| {
				let url_copy = ep.clone();
				let result = rt.block_on(self.download_episode(ep));
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

	fn sanitize_url(mut url: &str) -> &str {
		url = remove_newline(url);
		url
	}

	/// Download media from url to a Vec of optional EpisodeData.
	pub async fn download(&self, url: impl AsRef<str>) -> Result<EpisodeCollection> {
		let url = Self::sanitize_url(url.as_ref());
		Downloader::verify_url(url).await?;
		let url_type = URLType::get(url)?;
		match url_type {
			URLType::Playlist => {
				Ok::<EpisodeCollection, Box<dyn std::error::Error>>(self.download_show(url).await?)
			}
			URLType::Video => Ok::<EpisodeCollection, Box<dyn std::error::Error>>(vec![Some(
				self.download_episode(url).await?,
			)]),
		}
	}
}
