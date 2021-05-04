use crate::cacher::{cache_token, get_or_cache_token};
use crate::error::{OkOrGeneric, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub struct VideoInfo<'a> {
	pub name: &'a str,
	pub id: &'a str,
}

pub struct Requester {
	net: Client,
	token: Arc<Mutex<String>>,
}

impl Requester {
	pub async fn new<'a>() -> Result<Requester> {
		let net = Client::new();
		let token = Arc::new(Mutex::new(
			get_or_cache_token(async || Requester::get_auth_token(&net).await).await?,
		));
		Ok(Requester { net, token })
	}

	async fn get_auth_token<'b>(net: &Client) -> Result<String> {
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
			return Err(format!("Status code was not 200 OK.\nCode: {}", &status).into());
		}

		let text = response.text().await?;
		let json = serde_json::from_str::<serde_json::Value>(&text)?;
		let root = json.get(0).ok_or_generic("Could not get JSON value.")?;
		let token = root["value"]
			.as_str()
			.ok_or_generic("Could not get JSON value as str.")?;
		Ok(token.into())
	}

	async fn refresh_token(&self) -> Result<()> {
		let token: Option<String>;
		{
			let token_lock = self.token.try_lock().ok();
			if token_lock.is_none() {
				// If token is locked. (already refreshing)
				let lock = self.token.lock().unwrap();
				drop(lock);
				return Ok(());
			}
			{
				let guard = token_lock.unwrap();
				token = Some(guard.clone());
			}
		}
		let token = token.unwrap();

		println!("Refreshing auth token...");
		const REFRESH_ENDPOINT: &str =
			"https://isl.dr-massive.com/api/authorization/refresh?ff=idp%2Cldp%2Crpt&lang=da";
		let mut headers = header::HeaderMap::new();
		headers.append("Content-Type", "application/json".parse()?);
		let response = self
			.net
			.post(REFRESH_ENDPOINT)
			.headers(headers)
			.body(format!("{{ \"token\": \"{}\"}}", token))
			.send()
			.await?;

		let status = response.status();
		if status != StatusCode::OK {
			return Err(format!("Status code was not 200 OK.\nCode: {}", status).into());
		}

		let text = response.text().await?;
		let json = serde_json::from_str::<serde_json::Value>(&text)?;
		let val = json["value"]
			.as_str()
			.ok_or_generic("Could not get JSON value.")?;
		let token = val;
		cache_token(&token).ok();
		let mut token_ref = self.token.lock().unwrap();
		*token_ref = token.to_owned();
		println!("Refreshed auth token. Resuming...");
		Ok(())
	}

	async fn get_video_id(url: &str) -> Result<&str> {
		let id_start = url
			.rfind('_')
			.ok_or_generic("Could not find video id seperator.")?
			+ 1;
		let mut id_end = url.len();
		// Remove newline.
		url.chars().rev().enumerate().for_each(|(i, ch)| {
			if i > 2 {
				return;
			}
			if ch == '\r' || ch == '\n' {
				id_end -= 1;
			}
		});
		Ok(&url[id_start..id_end])
	}

	async fn get_video_name(url: &str) -> Result<&str> {
		let slash_start = url
			.rfind('/')
			.ok_or_generic("Could not find video name start seperator.")?
			+ 1;
		let slash_end = url
			.rfind('_')
			.ok_or_generic("Could not find video name end seperator.")?;
		Ok(&url[slash_start..slash_end])
	}

	pub async fn get_video_info(url: &str) -> Result<VideoInfo<'_>> {
		let name = Self::get_video_name(&url).await?;
		let id = Self::get_video_id(&url).await?;
		Ok(VideoInfo { name, id })
	}

	async fn construct_query_url(id: &str) -> Result<String> {
		const QUERY_URL_1: &str = "https://isl.dr-massive.com/api/account/items/";
		const QUERY_URL_2: &str = "/videos?delivery=stream&device=web_browser&ff=idp%2Cldp%2Crpt&lang=da&resolution=HD-1080&sub=Anonymous";

		let mut url = String::with_capacity(QUERY_URL_1.len() + id.len() + QUERY_URL_2.len());
		url.push_str(QUERY_URL_1);
		url.push_str(id);
		url.push_str(QUERY_URL_2);
		Ok(url)
	}

	fn parse_playlist_path_from_url(playlist_url: &str) -> Result<String> {
		let split = playlist_url.split("drtv");
		let trail = split
			.last()
			.ok_or_generic("Could not get the last element of split in playlist url.")?;
		let path = trail.replace('/', "%2F");
		let path = path.replace(' ', "%20");
		Ok(path)
	}

	pub async fn get_playlist_videos(&self, playlist_url: &str) -> Result<Vec<String>> {
		const PLAYLIST_INFO_URL_1: &str = "https://www.dr-massive.com/api/page?device=web_browser&ff=idp%2Cldp%2Crpt&geoLocation=dk&isDeviceAbroad=false&item_detail_expand=children&lang=da&list_page_size=24&max_list_prefetch=3&path=";
		const PLAYLIST_INFO_URL_2: &str =
			"&segments=drtv%2Coptedin&sub=Anonymous&text_entry_format=html";

		let mut url = String::with_capacity(
			PLAYLIST_INFO_URL_1.len() + playlist_url.len() / 2 + PLAYLIST_INFO_URL_2.len(),
		);
		let path = Self::parse_playlist_path_from_url(playlist_url)?;
		url.push_str(PLAYLIST_INFO_URL_1);
		url.push_str(&path);
		url.push_str(PLAYLIST_INFO_URL_2);

		let response = self.net.get(url).send().await?;
		let text = response.text().await?;
		let json: Value = serde_json::from_str(&text)?;
		let playlist = &json["item"];
		let eps_precursor = &playlist["episodes"];
		let eps_root = &eps_precursor["items"];

		let eps = eps_root
			.as_array()
			.ok_or_generic("Could not convert eps_root to an array.")?;
		let ep_links = eps
			.iter()
			.map(|x| {
				let mut path = x["watchPath"].as_str().unwrap().to_owned();
				path.insert_str(0, "https://www.dr.dk/drtv");
				path
			})
			.collect::<Vec<String>>();
		Ok(ep_links)
	}

	#[async_recursion::async_recursion]
	pub async fn get_media_url<'b>(&self, video_id: &str) -> Result<String> {
		let url = Self::construct_query_url(video_id).await?;

		let token: Option<String>;
		{
			let mutex_val = self.token.lock().unwrap();
			token = Some(mutex_val.clone());
		}
		let token = token.ok_or_generic("Could not get token from mutex.")?;

		let result = self.net.get(url).bearer_auth(token).send().await?;

		let status = result.status();
		if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
			self.refresh_token().await?;
			return self.get_media_url(video_id).await;
		}
		if status != StatusCode::OK {
			return Err(format!("Status code was not 200 OK.\nCode: {}", status).into());
		}

		let text = result.text().await?;
		let json: Value = serde_json::from_str(&text)?;
		let root = json.get(0).ok_or_generic("Could not get JSON value.")?;
		let url = root["url"]
			.as_str()
			.ok_or_generic("Could not get 'url' from root as str.")?;
		Ok(String::from(url))
	}
}
