#![feature(try_trait)]
#![feature(async_closure)]

mod cacher;
mod downloader;
mod error;
mod requester;

use downloader::Downloader;
use requester::Result;
use std::io::{stdin, stdout, Write};

fn clear_console() {
	print!("\x1B[2J\x1B[1;1H");
}

#[tokio::main]
async fn main() -> Result<'static, ()> {
	let mut downloader = Downloader::new(requester::Requester::new().await?);
	let inp = stdin();
	let mut out = stdout();
	let mut video_num: u8 = 0;
	loop {
		clear_console();
		out.write_all(b"Enter url: ")?;
		out.flush()?;
		let mut input_url = String::new();
		inp.read_line(&mut input_url)?;
		downloader
			.download(format!("./video_{}.m3u8", video_num), &input_url)
			.await?;
		video_num += 1;
		out.write_all(b"Done!")?;
		out.flush()?;
		std::thread::sleep(std::time::Duration::from_millis(2000));
	}
}
