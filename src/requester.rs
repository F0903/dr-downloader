use crate::cacher::{cache_token, get_or_cache_token};
use crate::error::{OkOrGeneric, Result};
use crate::util::{find_char, remove_newline, rfind_char};
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct EpisodeInfo<'a> {
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
		let lock_result = self.token.try_lock();
		if lock_result.is_err() {
			// If token is locked. (already refreshing)
			let _lock = self.token.lock().await; // Wait for refresh
			return Ok(());
		}
		let mut token = lock_result.unwrap();

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
		cache_token(&val).ok();
		*token = val.to_owned();
		println!("Refreshed auth token.\nResuming...");
		Ok(())
	}

	async fn get_episode_id(url: &str) -> Result<&str> {
		let new_url = remove_newline(url);
		let id_start = new_url
			.rfind('_')
			.ok_or_generic("Could not find episode id seperator.")?
			+ 1;
		let mut id_end = find_char(new_url, '/', id_start, new_url.len()).unwrap_or(0);
		if id_end == 0 || id_end <= id_start {
			id_end = new_url.len();
		}
		Ok(&new_url[id_start..id_end])
	}

	async fn get_episode_name(url: &str) -> Result<&str> {
		let new_url = remove_newline(url);
		let mut name_start = new_url
			.rfind('/')
			.ok_or_generic("Could not find episode name seperator.")?
			+ 1;
		if name_start == new_url.len() {
			name_start = rfind_char(new_url, '/', 1, new_url.len() - 1)?;
		}
		let mut name_end = find_char(new_url, '/', name_start, new_url.len()).unwrap_or(0);
		if name_end == 0 || name_end <= name_start {
			name_end = new_url.len();
		}
		Ok(&new_url[name_start..name_end])
	}

	pub async fn get_episode_info(url: &str) -> Result<EpisodeInfo<'_>> {
		let name = Self::get_episode_name(&url).await?;
		let id = Self::get_episode_id(&url).await?;
		Ok(EpisodeInfo { name, id })
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

	fn parse_show_path_from_url(playlist_url: &str) -> Result<String> {
		let split = playlist_url.split("drtv");
		let trail = split
			.last()
			.ok_or_generic("Could not get the last element of split in show url.")?;
		let path = trail.replace('/', "%2F");
		let path = path.replace(' ', "%20");
		Ok(path)
	}

	pub async fn get_show_episodes(&self, playlist_url: &str) -> Result<Vec<String>> {
		const PLAYLIST_INFO_URL_1: &str = "https://www.dr-massive.com/api/page?device=web_browser&ff=idp%2Cldp%2Crpt&geoLocation=dk&isDeviceAbroad=false&item_detail_expand=children&lang=da&list_page_size=24&max_list_prefetch=3&path=";
		const PLAYLIST_INFO_URL_2: &str =
			"&segments=drtv%2Coptedin&sub=Anonymous&text_entry_format=html";

		let mut url = String::with_capacity(
			PLAYLIST_INFO_URL_1.len() + playlist_url.len() / 2 + PLAYLIST_INFO_URL_2.len(),
		);
		let path = Self::parse_show_path_from_url(playlist_url)?;
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
		let token = self.token.lock().await;
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
