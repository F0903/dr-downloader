use std::fs;
use std::io::{ErrorKind, Result, Write};
use std::process::{Command, Stdio};

const FFMPEG: &[u8] = include_bytes!("../ffmpeg-win32.exe");

pub struct Converter {
	ffmpeg_path: Option<String>,
}

impl Converter {
	pub fn new() -> Result<Self> {
		let dir = std::env::temp_dir().join("ffmpeg.exe");
		let dir_str = dir.to_str().ok_or(ErrorKind::InvalidData)?;
		fs::write(&dir_str, FFMPEG)?;
		Ok(Converter {
			ffmpeg_path: Some(dir_str.to_owned()),
		})
	}

	pub fn convert<T: AsRef<str>>(&self, data: &[u8], out_path: T) -> Result<()> {
		let ffmpeg_path = self.ffmpeg_path.as_ref().ok_or(ErrorKind::NotFound)?;
		let out = out_path.as_ref();
		std::fs::write(out, "")?; // Create file first otherwise canonicalize wont work.
		let out = std::fs::canonicalize(out)?;
		let out = out.to_str().ok_or(ErrorKind::NotFound)?;
		let mut proc = Command::new(ffmpeg_path)
			.args(&[
				"-y",
				"-hide_banner",
				"-loglevel",
				"info",
				"-protocol_whitelist",
				"http,https,tcp,tls,crypto,data,file,pipe",
				"-i",
				"pipe:0",
				"-c",
				"copy",
				out,
			])
			.stdin(Stdio::piped())
			.stderr(Stdio::inherit())
			.stdout(Stdio::inherit())
			.spawn()?;
		{
			let mut inp = proc.stdin.take().unwrap();
			inp.write_all(&data)?;
			inp.flush()?;
		}
		proc.wait()?;
		Ok(())
	}
}
