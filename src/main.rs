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
use std::fs;
use std::io::stdin;

fn clear_console() {
	print!("\x1B[2J\x1B[1;1H");
}

fn get_video_num<'a>() -> Result<'a, u8> {
	let dir = fs::read_dir("./")?;
	let mut largest_num = 0;
	dir.for_each(|x| {
		if let Ok(entry) = x {
			let file_name = entry.file_name();
			let to_str = file_name.into_string();
			if let Ok(name) = to_str {
				if !name.contains("video") {
					return;
				}

				let split = name.rsplit('_').next().unwrap_or("0");
				let dot_index = split.find('.').unwrap_or(1);
				let num_str = &split[..dot_index];
				let num = num_str.parse::<u8>().unwrap();
				if num > largest_num {
					largest_num = num;
				}
			}
		}
	});
	Ok(largest_num)
}

#[tokio::main]
async fn main() -> Result<'static, ()> {
	#[cfg(all(windows, not(debug_assertions)))]
	win32::set_virtual_console_mode();

	let mut input_url = String::new();
	let mut downloader = Downloader::new(
		requester::Requester::new().await?,
		converter::Converter::new()?,
	);
	let inp = stdin();
	let mut video_num: u8 = get_video_num().unwrap_or(0);
	loop {
		clear_console();
		fprint!("\x1B[1mEnter url:\x1B[0m ");

		inp.read_line(&mut input_url)?;
		let result = downloader
			.download(format!("./video_{}.mp4", video_num), &input_url)
			.await;

		if let Err(val) = result {
			fprintln!("\x1B[91mError!\x1B[0m {}", val);
			let trace = val.backtrace();
			if let Some(bt) = trace {
				std::fs::write("error.txt", bt.to_string()).ok();
			}
			const CLEAR_TIME: u16 = 5000;
			fprint!("Clearing in {}s", CLEAR_TIME / 1000);
			std::thread::sleep(std::time::Duration::from_millis(CLEAR_TIME as u64));
			continue;
		}
		video_num += 1;
		fprintln!("\x1B[92mDone!\x1B[0m");
		std::thread::sleep(std::time::Duration::from_millis(2000));
	}
}
