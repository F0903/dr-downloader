use std::io::{ErrorKind, Result, Write};
use std::process::{Command, Stdio};

pub struct Converter {
	ffmpeg_path: String,
}

impl Converter {
	/// Attempt to create a new Converter.
	pub fn new(ffmpeg_path: String) -> Self {
		Converter { ffmpeg_path }
	}

	/// Convert data to another format through FFMPEG.
	pub fn convert(&self, data: &[u8], out_path: impl AsRef<str>) -> Result<()> {
		let out_path = out_path.as_ref();
		std::fs::write(out_path, "")?; // Create file first otherwise canonicalize wont work.
		let out_path = std::fs::canonicalize(out_path)?;
		let out_path = out_path.to_str().ok_or(ErrorKind::NotFound)?;
		let mut proc = Command::new(&self.ffmpeg_path)
			.args(&[
				"-y",
				"-hide_banner",
				"-loglevel",
				"panic",
				"-protocol_whitelist",
				"http,https,tcp,tls,crypto,pipe",
				"-i",
				"pipe:0",
				"-c",
				"copy",
				out_path,
			])
			.stdin(Stdio::piped())
			.stderr(Stdio::inherit())
			.stdout(Stdio::inherit())
			.spawn()?;
		{
			// VERY IMPORTANT: This scope is needed because FFMPEG waits for this pipe to close. Removal will cause freezing.
			let mut inp = proc.stdin.take().ok_or(ErrorKind::BrokenPipe)?;
			inp.write_all(&data)?;
			inp.flush()?;
		}
		proc.wait()?;
		Ok(())
	}
}
