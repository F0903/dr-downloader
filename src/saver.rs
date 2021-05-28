use crate::converter::Converter;
use crate::downloader::Downloader;
use crate::error::Result;
use std::path;

/// A utility for downloading media to a path.
pub struct Saver<'a> {
	downloader: Downloader<'a>,
	converter: Option<Converter>,
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

	pub fn with_converter(mut self, converter: Converter, extension: &'a str) -> Self {
		self.converter = Some(converter);
		self.extension = extension;
		self
	}

	fn save_ep() {}

	async fn save_show(&self, show_url: impl AsRef<str>, out_dir: impl Into<String>) -> Result<()> {
		let show_url = show_url.as_ref();
		let out_dir = out_dir.into();
		for ep in self
			.downloader
			.download_show(show_url)
			.await?
			.iter()
			.flatten()
		{
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

	pub fn save(&self, url: impl AsRef<str>, out_dir: impl AsRef<str>) -> Result<()> {}
}

/*

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

*/
