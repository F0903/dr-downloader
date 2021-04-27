#![feature(try_trait)]
#![feature(async_closure)]
#![feature(backtrace)]

mod cacher;
mod converter;
mod downloader;
mod error;
mod requester;

#[cfg(all(windows, not(debug_assertions)))]
mod win32;

use downloader::Downloader;
use requester::Result;
use std::fs;
use std::io::{stdin, Stdin};

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

async fn do_stuff(
	inp: &Stdin,
	downloader: &mut Downloader,
	video_num: &mut u8,
) -> Result<'static, ()> {
	let mut input_url = String::new();
	inp.read_line(&mut input_url)?;
	downloader
		.download(format!("./video_{}.mp4", video_num), &input_url)
		.await?;
	*video_num += 1;
	Ok(())
}

#[tokio::main]
async fn main() -> Result<'static, ()> {
	#[cfg(all(windows, not(debug_assertions)))]
	win32::set_color_mode();

	let mut downloader = Downloader::new(
		requester::Requester::new().await?,
		converter::Converter::new()?,
	);
	let inp = stdin();
	let mut video_num: u8 = get_video_num().unwrap_or(0);
	loop {
		clear_console();
		println!("Enter url: ");
		let result = do_stuff(&inp, &mut downloader, &mut video_num).await;
		if let Err(val) = result {
			println!("Error! {}", val);
			let trace = val.backtrace();
			if let Some(bt) = trace {
				std::fs::write("error.txt", bt.to_string()).ok();
			}
			std::thread::sleep(std::time::Duration::from_millis(10000));
			continue;
		}
		println!("Done!");
		std::thread::sleep(std::time::Duration::from_millis(3000));
	}
}
