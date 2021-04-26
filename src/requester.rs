use crate::cacher::get_or_cache_token;
use crate::error::GenericError;
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use std::error::Error;

type ErrorType = Box<dyn Error>;
pub type Result<'a, T, E = ErrorType> = std::result::Result<T, E>;

pub struct Requester {
	net: Client,
	token: String,
}

impl Requester {
	pub async fn new<'a>() -> Result<'a, Requester> {
		let net = Client::new();
		let token = get_or_cache_token(async || Requester::get_auth_token(&net).await).await?;
		Ok(Requester { net, token })
	}

	async fn get_auth_token<'b>(net: &Client) -> Result<'_, String> {
		println!("Getting auth token...");
		const AUTH_ENDPOINT: &str = "https://isl.dr-massive.com/api/authorization/anonymous-sso?device=web_browser&ff=idp%2Cldp%2Crpt&lang=da";
		let mut headers = header::HeaderMap::new();
		headers.append(
			"Content-Type",
			header::HeaderValue::from_static("application/json"),
		);
		let response = net
			.post(AUTH_ENDPOINT)
			.headers(headers)
			.body("{\"deviceId\":\"632bdbff-d073-4b6c-85cb-76a0de00506d\",\"scopes\":[\"Catalog\"],\"optout\":true,\"cookieType\":\"Session\"}")
			.send()
			.await?;

		let status = response.status();
		if status != StatusCode::OK {
			return Err(
				GenericError(format!("Status code was not 200 OK.\nCode: {}", &status)).into(),
			);
		}

		let text = response.text().await?;
		let json = serde_json::from_str::<serde_json::Value>(&text)?;
		let root = json
			.get(0)
			.ok_or_else(|| GenericError("Could not get JSON value.".into()))?;
		let token = root["value"]
			.as_str()
			.ok_or_else(|| GenericError("Could not get JSON value as str.".into()))?;
		Ok(token.into())
	}

	async fn refresh_token(&mut self) -> Result<'_, ()> {
		println!("Refreshing auth token...");
		const REFRESH_ENDPOINT: &str =
			"https://isl.dr-massive.com/api/authorization/refresh?ff=idp%2Cldp%2Crpt&lang=da";
		let mut headers = header::HeaderMap::new();
		headers.append("Content-Type", "application/json".parse()?);
		let response = self
			.net
			.post(REFRESH_ENDPOINT)
			.headers(headers)
			.body(format!("{{ \"token\": \"{}\"}}", self.token))
			.send()
			.await?;

		let status = response.status();
		if status != StatusCode::OK {
			return Err(
				GenericError(format!("Status code was not 200 OK.\nCode: {}", status)).into(),
			);
		}

		let text = response.text().await?;
		let json = serde_json::from_str::<serde_json::Value>(&text)?;
		let root = json
			.get(0)
			.ok_or_else(|| GenericError("Could not get JSON value.".into()))?;
		let val = root["value"]
			.as_str()
			.ok_or_else(|| GenericError("Could not get JSON value.".into()))?;
		self.token = val.into();
		Ok(())
	}

	pub async fn get_video_id(url: &str) -> Result<'_, &str> {
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
		const URL_FIRST_HALF: &str = "https://isl.dr-massive.com/api/account/items/";
		const URL_SECOND_HALF: &str = "/videos?delivery=stream&device=web_browser&ff=idp%2Cldp%2Crpt&lang=da&resolution=HD-1080&sub=Anonymous";

		let mut string =
			String::with_capacity(URL_FIRST_HALF.len() + id.len() + URL_SECOND_HALF.len());
		string.push_str(URL_FIRST_HALF);
		string.push_str(id);
		string.push_str(URL_SECOND_HALF);
		Ok(string)
	}

	#[async_recursion::async_recursion]
	pub async fn get_media_url<'b>(&mut self, video_id: &str) -> Result<'b, String> {
		println!("Sending request...");
		let url = Self::construct_query_url(video_id).await?;
		let result = self.net.get(url).bearer_auth(&self.token).send().await?;

		let status = result.status();
		if status != StatusCode::OK {
			return Err(
				GenericError(format!("Status code was not 200 OK.\nCode: {}", status)).into(),
			);
		}
		if status == StatusCode::UNAUTHORIZED {
			self.refresh_token().await?;
			return self.get_media_url(video_id).await;
		}

		let text = result.text().await?;
		let json: Value = serde_json::from_str(&text)?;
		let root = json
			.get(0)
			.ok_or_else(|| GenericError("Could not get JSON value.".into()))?;
		let url = root["url"]
			.as_str()
			.ok_or_else(|| GenericError("Could not get 'url' from root as str.".into()))?;
		Ok(String::from(url))
	}
}
