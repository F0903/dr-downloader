use crate::converter::Converter;
use crate::downloader::Downloader;
use crate::error::ok_or_generic::OkOrGeneric;
use crate::error::Result;
use crate::models::URLType;
use crate::util::{legalize_filename, remove_newline_string};
use std::path;

/// A utility for downloading media to a path.
pub struct Saver<'a> {
    downloader: Downloader<'a>,
    converter: Option<Converter<'a>>,
    extension: String,
}

impl<'a> Saver<'a> {
    pub fn new(downloader: Downloader<'a>) -> Self {
        Saver {
            downloader,
            converter: None,
            extension: ".m3u8".to_owned(),
        }
    }

    pub fn with_converter(
        mut self,
        converter: Converter<'a>,
        extension: impl Into<String>,
    ) -> Self {
        self.converter = Some(converter);
        let extension = extension.into();
        self.extension = if !extension.starts_with('.') {
            let mut temp = extension.to_owned();
            temp.insert(0, '.');
            temp
        } else {
            extension
        };
        self
    }

    async fn save_ep(&self, ep_url: String, out_dir: impl AsRef<str>) -> Result<()> {
        let out_dir = out_dir.as_ref();
        let ep = self.downloader.download_episode(ep_url).await?;
        let mut path = path::PathBuf::from(out_dir);
        let legal_name = crate::util::legalize_filename(&ep.info.name);
        path.push(format!("{}{}", legal_name, self.extension));
        if let Some(con) = &self.converter {
            con.convert(&ep.data, path.to_str().ok_or_generic("Path was invalid.")?)?;
        } else {
            std::fs::write(path, &ep.data)?;
        }
        Ok(())
    }

    async fn save_show(&self, show_url: String, out_dir: impl AsRef<str>) -> Result<()> {
        let out_dir = out_dir.as_ref();
        for ep in self
            .downloader
            .download_show(show_url)
            .await?
            .iter()
            .flatten()
        {
            let mut path = path::PathBuf::from(&out_dir);
            let legal_name = legalize_filename(&ep.info.name);
            path.push(format!("{}{}", legal_name, self.extension));
            if let Some(con) = &self.converter {
                con.convert(&ep.data, path.to_str().ok_or_generic("Path was invalid.")?)?;
            } else {
                std::fs::write(path, &ep.data)?;
            }
        }
        Ok(())
    }

    fn sanitize_url(url: &mut String) {
        remove_newline_string(url);
    }

    pub async fn save(&self, url: impl Into<String>, out_dir: impl AsRef<str>) -> Result<()> {
        let mut url = url.into();
        Self::sanitize_url(&mut url);
        let url_type = URLType::get(&url)?;
        match url_type {
            URLType::Video => self.save_ep(url, out_dir).await,
            URLType::Playlist => self.save_show(url, out_dir).await,
        }
    }
}
