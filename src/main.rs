#![feature(try_trait)]

use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::error::Error;
use std::fs::write;
use std::io::{stdin, stdout, BufRead, Write};

const URL_FIRST_HALF: &str = "https://isl.dr-massive.com/api/account/items/";
const URL_SECOND_HALF: &str = "/videos?delivery=stream&device=web_browser&ff=idp%2Cldp%2Crpt&lang=da&resolution=HD-1080&sub=Anonymous";

//NOTE: Currently, the auth token needs to be manually extracted from the browser, and put in auth.txt
//TODO: Find a way to get an updated auth token from dr
const AUTH: &str = include_str!("auth.txt");

type Result<'a, T, E = Box<dyn Error>> = std::result::Result<T, E>;

#[derive(Debug)]
struct GenericError<'a>(&'a str);

impl<'a> std::fmt::Display for GenericError<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(self.0)
	}
}

impl<'a> std::error::Error for GenericError<'a> {}

impl<'a> std::convert::From<std::option::NoneError> for GenericError<'a> {
	fn from(_err: std::option::NoneError) -> Self {
		GenericError("Value was None.")
	}
}

fn clear_console() {
	print!("\x1B[2J\x1B[1;1H");
}

async fn get_video_id(url: &str) -> Result<'_, &str> {
	let index = url.rfind('_').unwrap() + 1;
	let mut end = url.len();
	url.chars().rev().enumerate().for_each(|(i, ch)| {
		if i > 2 {
			return;
		}
		if ch == '\r' || ch == '\n' {
			end -= 1;
		}
	});
	let id = &url[index..end];
	Ok(id)
}

async fn construct_query_url(id: &str) -> Result<'_, String> {
	let mut string = String::with_capacity(URL_FIRST_HALF.len() + id.len() + URL_SECOND_HALF.len());
	string.push_str(URL_FIRST_HALF);
	string.push_str(id);
	string.push_str(URL_SECOND_HALF);
	Ok(string)
}

async fn get_video_url<'a>(net: &Client, video_id: &str) -> Result<'a, String> {
	let url = construct_query_url(video_id).await?;
	let result = net.get(url).bearer_auth(AUTH).send().await?;
	let status = result.status();
	if status != StatusCode::OK {
		panic!("Status code was not 200 OK.\nCode: {}", status);
	}
	let text = result.text().await?;
	let json: Value = serde_json::from_str(&text)?;
	let root = json
		.get(0)
		.ok_or(GenericError("Could not get JSON value."))?;
	let url = root["url"]
		.as_str()
		.ok_or(GenericError("Could not get 'url' from root as str."))?;
	Ok(String::from(url))
}

async fn get_file(url: &str) -> Result<'_, String> {
	let result = reqwest::get(url).await?;
	let status = result.status();
	if status != StatusCode::OK {
		panic!("Status code was not 200 OK.\nCode: {}", status);
	}
	let text = result.text().await?;
	Ok(text)
}

#[tokio::main]
async fn main() -> Result<'static, ()> {
	let net = Client::new();
	let inp = stdin();
	let mut out = stdout();
	let mut video_num: u8 = 0;
	loop {
		clear_console();
		out.write_all(b"Enter url: ")?;
		out.flush()?;
		let mut input_url = String::new();
		inp.read_line(&mut input_url)?;
		let id = get_video_id(&input_url).await?;
		let url = get_video_url(&net, id).await?;
		let content = get_file(&url).await?;
		write(format!("./video_{}.m3u8", video_num), content)?;
		video_num += 1;
		out.write_all(b"Done!")?;
		out.flush()?;
		std::thread::sleep(std::time::Duration::from_millis(2000));
	}
}
