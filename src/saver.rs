use crate::converter::Converter;
use crate::downloader::Downloader;
use crate::error::ok_or_generic::OkOrGeneric;
use crate::error::Result;
use crate::models::URLType;
use std::path;

/// A utility for downloading media to a path.
pub struct Saver<'a> {
	downloader: Downloader<'a>,
	converter: Option<Converter<'a>>,
	extension: &'a str,
}

impl<'a> Saver<'a> {
	pub fn new(downloader: Downloader<'a>) -> Self {
		Saver {
			downloader,
			converter: None,
			extension: "mp4",
		}
	}

	pub fn with_converter(mut self, converter: Converter<'a>, extension: &'a str) -> Self {
		self.converter = Some(converter);
		self.extension = extension;
		self
	}

	async fn save_ep(&self, ep_url: String, out_dir: impl AsRef<str>) -> Result<()> {
		let out_dir = out_dir.as_ref();
		let ep = self.downloader.download_episode(ep_url).await?;
		let mut path = path::PathBuf::from(out_dir);
		path.push(format!("./{}.mp4", ep.info.name));
		if let Some(con) = &self.converter {
			con.convert(&ep.data, path.to_str().ok_or_generic("Path was invalid.")?)?;
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
			path.push(format!("./{}.mp4", ep.info.name));
			if let Some(con) = &self.converter {
				con.convert(&ep.data, path.to_str().ok_or_generic("Path was invalid.")?)?;
			}
		}
		Ok(())
	}

	pub async fn save(&self, url: impl Into<String>, out_dir: impl AsRef<str>) -> Result<()> {
		let url = url.into();
		let url_type = URLType::get(&url)?;
		match url_type {
			URLType::Video => self.save_ep(url, out_dir).await,
			URLType::Playlist => self.save_show(url, out_dir).await,
		}
	}
}