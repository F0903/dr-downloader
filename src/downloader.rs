use crate::error::Result;
use crate::event::Event;
use crate::models::{episode::EpisodeData, URLType};
use crate::requester::Requester;
use crate::util::remove_newline;
use rayon::prelude::*;
use reqwest::StatusCode;
use std::borrow::Cow;

lazy_static! {
    static ref DR_EP_URL_REGEX: regex::Regex =
        regex::Regex::new(r#"(((https)|(http))(://www\.dr\.dk/drtv/)).*_\d+"#).unwrap();
}

pub type EpisodeCollection = Vec<Option<EpisodeData>>;

pub struct Downloader<'a> {
    requester: Requester,
    pub download_event: Event<'a, Cow<'a, str>>,
    pub finished_event: Event<'a, Cow<'a, str>>,
    pub failed_event: Event<'a, Cow<'a, str>>,
}

impl<'a> Default for Downloader<'a> {
    /// Create a default Downloader.
    fn default() -> Self {
        let rt = tokio::runtime::Handle::current();
        let requester = rt
            .block_on(Requester::new())
            .expect("Could not wait for requester future. Try using Downloader::new() manually.");
        Self::new(requester)
    }
}

impl<'a> Downloader<'a> {
    /// Create a new Downloader.
    pub fn new(requester: Requester) -> Self {
        Downloader {
            requester,
            download_event: Event::new(),
            finished_event: Event::new(),
            failed_event: Event::new(),
        }
    }

    pub async fn default_async() -> Result<Downloader<'a>> {
        Ok(Self::new(Requester::new().await?))
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

    pub(crate) async fn download_episode(&self, ep_url: String) -> Result<EpisodeData> {
        self.download_event.call(Cow::Owned(ep_url.clone()));
        let info = Requester::get_episode_info(ep_url.clone()).await?;
        let url = self.requester.get_episode_url(&info.id).await?;
        let content = Self::get_as_string(&url).await?;
        self.finished_event.call(Cow::Owned(ep_url));
        Ok(EpisodeData {
            info,
            data: content.as_bytes().into(),
        })
    }

    pub(crate) async fn download_show(&self, show_url: String) -> Result<EpisodeCollection> {
        self.download_event.call(Cow::Owned(show_url.clone()));
        let eps = self.requester.get_show_episodes(&show_url).await?;
        let rt = tokio::runtime::Handle::current();
        let show_data = eps
            .into_par_iter()
            .map(|ep| {
                let url_copy = ep.clone();
                let result = rt.block_on(self.download_episode(ep));
                match result {
                    Ok(data) => {
                        self.finished_event.call(Cow::Owned(url_copy));
                        Some(data)
                    }
                    Err(_) => {
                        self.failed_event.call(Cow::Owned(url_copy));
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
        let url = String::from(Self::sanitize_url(url.as_ref()));
        Downloader::verify_url(&url).await?;
        let url_type = URLType::get(&url)?;
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
