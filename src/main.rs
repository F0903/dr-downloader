#![feature(try_trait)]
#![feature(async_closure)]
#![feature(backtrace)]

#[macro_use]
extern crate lazy_static;

mod cacher;
mod converter;
mod downloader;
mod error;
#[macro_use]
mod printutil;
mod requester;

#[cfg(all(windows, not(debug_assertions)))]
mod win32;

use downloader::Downloader;
use requester::Result;
use std::io::stdin;

fn clear_console() {
	print!("\x1B[2J\x1B[1;1H");
}

fn log_error(err: impl AsRef<dyn std::error::Error>) {
	fprintln!("\x1B[91mError!\x1B[0m {}", err.as_ref());
	let trace = err.as_ref().backtrace();
	let content = match trace {
		Some(val) => val.to_string(),
		None => err.as_ref().to_string(),
	};
	std::fs::write("error.txt", content).ok();
}

#[tokio::main]
async fn main() -> Result<()> {
	#[cfg(all(windows, not(debug_assertions)))]
	win32::set_virtual_console_mode();

	let downloader = Box::leak::<'static>(Box::new(Downloader::new(
		requester::Requester::new().await?,
		converter::Converter::new()?,
	)));

	let inp = stdin();
	let mut input_url = String::new();
	loop {
		clear_console();
		fprint!("\x1B[1mEnter url:\x1B[0m ");

		inp.read_line(&mut input_url)?;
		let result = downloader.download("./", &input_url).await;

		if let Err(val) = result {
			log_error(val);
			const CLEAR_TIME: u16 = 5000;
			fprint!("Clearing in {}s", CLEAR_TIME / 1000);
			std::thread::sleep(std::time::Duration::from_millis(CLEAR_TIME as u64));
			continue;
		}

		fprintln!("\x1B[92mDone!\x1B[0m");
		std::thread::sleep(std::time::Duration::from_millis(2000));
	}
}
