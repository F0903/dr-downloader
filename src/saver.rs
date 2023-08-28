use crate::converter::Converter;
use crate::downloader::Downloader;
use crate::error::ok_or_generic::OkOrGeneric;
use crate::error::Result;
use crate::format::Format;
use crate::models::URLType;
use crate::util::{legalize_filename, remove_newline_string};
use std::path;

const DEFAULT_FORMAT: Format = Format::from_exact_extension(".mp4");

/// A utility for downloading media to a path.
#[derive(Clone)]
pub struct Saver<'a> {
    downloader: Downloader<'a>,
    converter: Option<Converter<'a>>,
}

impl<'a> Saver<'a> {
    pub fn new(downloader: Downloader<'a>) -> Self {
        Saver {
            downloader,
            converter: None,
        }
    }

    pub fn with_converter(mut self, converter: Converter<'a>) -> Self {
        self.converter = Some(converter);
        self
    }

    async fn save_ep<'b>(
        &self,
        ep_url: String,
        out_dir: impl AsRef<str>,
        format: Format<'b>,
    ) -> Result<()> {
        let out_dir = out_dir.as_ref();
        let requester = self.downloader.get_requester();

        let ep_info = requester.get_episode_info(&ep_url).await?;
        let ep_url = requester.get_episode_url(&ep_info.id).await?;
        let mut path = path::PathBuf::from(out_dir);
        let legal_name = crate::util::legalize_filename(&ep_info.name);
        path.push(format!("{}{}", legal_name, format.get_extension()));
        if let Some(con) = &self.converter {
            con.convert(ep_url, path.to_str().ok_or_generic("Path was invalid.")?)?;
        } else {
            let ep = self.downloader.download_episode(ep_url).await?;
            std::fs::write(path, &ep.data)?;
        }
        Ok(())
    }

    async fn save_show<'b>(
        &self,
        show_url: String,
        out_dir: impl AsRef<str>,
        format: Format<'b>,
    ) -> Result<()> {
        let out_dir = out_dir.as_ref();
        let requester = self.downloader.get_requester();

        for ep_url in requester.get_show_episodes(&show_url).await? {
            let ep_info = requester.get_episode_info(&ep_url).await?;

            let mut path = path::PathBuf::from(&out_dir);
            let legal_name = legalize_filename(&ep_info.name);
            path.push(format!("{}{}", legal_name, format.get_extension()));
            if let Some(con) = &self.converter {
                con.convert(ep_url, path.to_str().ok_or_generic("Path was invalid.")?)?;
            } else {
                let ep = self.downloader.download_episode(ep_url).await?;
                std::fs::write(path, &ep.data)?;
            }
        }
        Ok(())
    }

    fn sanitize_url(url: &mut String) {
        remove_newline_string(url);
    }

    /// Download media to file in directory. If the Saver has no Converter specified the format argument is ignored.
    pub async fn save<'b>(
        &self,
        url: impl Into<String>,
        out_dir: impl AsRef<str>,
        format: Option<Format<'b>>,
    ) -> Result<()> {
        let mut url = url.into();
        Self::sanitize_url(&mut url);
        let url_type = URLType::get(&url)?;
        let format = format.unwrap_or(DEFAULT_FORMAT);
        match url_type {
            URLType::Video => self.save_ep(url, out_dir, format).await,
            URLType::Playlist => self.save_show(url, out_dir, format).await,
        }
    }
}
