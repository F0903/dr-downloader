use crate::error::Result;
use crate::event::Event;
use std::borrow::Cow;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct Converter<'a> {
    ffmpeg_path: String,
    pub on_convert: Event<'a, Cow<'a, str>>,
    pub on_done: Event<'a, Cow<'a, str>>,
}

impl<'a> Converter<'a> {
    /// Attempt to create a new Converter.
    pub fn new(ffmpeg_path: String) -> Self {
        Converter {
            ffmpeg_path,
            on_convert: Event::new(),
            on_done: Event::new(),
        }
    }

    /// Convert data to another format through FFMPEG.
    pub fn convert(&self, input_url: impl AsRef<str>, out_path: impl AsRef<str>) -> Result<()> {
        let out_path = out_path.as_ref();
        std::fs::File::create(out_path)?; // Create file first otherwise canonicalize wont work.
        let out_path = std::fs::canonicalize(out_path)?;
        let out_path = out_path.to_str().ok_or("Invalid output path.")?;
        self.on_convert.call(Cow::Owned(out_path.to_owned()));
        let mut proc = Command::new(&self.ffmpeg_path)
            .args(&[
                "-y",
                "-hide_banner",
                "-loglevel",
                "info",
                "-protocol_whitelist",
                "http,https,tcp,tls,crypto,pipe",
                "-i",
                input_url.as_ref(),
                "-c",
                "copy",
                out_path,
            ])
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .spawn()
            .map_err(|_| {
                "Could not start FFmpeg. Please install and copy to downloader root, or add to PATH."
                    .to_owned()
            })?;
        proc.wait()?;
        self.on_done.call(Cow::Owned(out_path.to_owned()));
        Ok(())
    }
}
