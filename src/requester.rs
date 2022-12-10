use crate::cacher::{cache_token, get_or_cache_token};
use crate::error::{OkOrGeneric, Result};
use crate::models::episode::EpisodeInfo;
use crate::util::{find_char, rfind_char};
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Requester {
    net: Client,
    token: Arc<Mutex<String>>,
}

impl Requester {
    pub async fn new<'a>() -> Result<Requester> {
        let net = Client::new();
        let token = Arc::new(Mutex::new(
            get_or_cache_token(|| Requester::get_auth_token(&net)).await?,
        ));
        Ok(Requester { net, token })
    }

    async fn get_auth_token<'b>(net: &Client) -> Result<String> {
        const AUTH_ENDPOINT: &str = "https://production.dr-massive.com/api/authorization/anonymous-sso?device=web_browser&ff=idp%2Cldp%2Crpt&lang=da";
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

        const REFRESH_ENDPOINT: &str =
            "https://production.dr-massive.com/api/authorization/refresh?ff=idp%2Cldp%2Crpt&lang=da";
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
        Ok(())
    }

    fn construct_show_query_url(show_url: &str) -> Result<String> {
        let path = Self::parse_show_path_from_url(show_url)?;
        let url = format!("https://www.dr-massive.com/api/page?device=web_browser&ff=idp%2Cldp%2Crpt&geoLocation=dk&isDeviceAbroad=false&item_detail_expand=children&lang=da&list_page_size=24&max_list_prefetch=3&path={}&segments=drtv%2Coptedin&sub=Anonymous&text_entry_format=html", path);
        Ok(url)
    }

    fn parse_show_path_from_url(show_url: &str) -> Result<String> {
        let split = show_url.split("drtv");
        let trail = split
            .last()
            .ok_or_generic("Could not get the last element of split in show url.")?;
        let path = trail.replace('/', "%2F");
        let path = path.replace(' ', "%20");
        Ok(path)
    }

    async fn construct_ep_query_url(ep_id: &str) -> Result<String> {
        let url = format!("https://production.dr-massive.com/api/account/items/{}/videos?delivery=stream&device=web_browser&ff=idp%2Cldp%2Crpt&lang=da&resolution=HD-1080&sub=Anonymous", ep_id);
        Ok(url)
    }

    async fn parse_episode_name(url: &str) -> Result<&str> {
        let mut name_start = url
            .rfind('/')
            .ok_or_generic("Could not find episode name seperator.")?
            + 1;
        if name_start == url.len() {
            name_start = rfind_char(url, '/', 1, url.len() - 1)?;
        }
        let mut name_end = find_char(url, '/', name_start, url.len()).unwrap_or(0);
        if name_end == 0 || name_end <= name_start {
            name_end = url.len();
        }
        Ok(&url[name_start..name_end])
    }

    async fn parse_episode_id(url: &str) -> Result<&str> {
        let id_start = url
            .rfind('_')
            .ok_or_generic("Could not find episode id seperator.")?
            + 1;
        let mut id_end = find_char(url, '/', id_start, url.len()).unwrap_or(0);
        if id_end == 0 || id_end <= id_start {
            id_end = url.len();
        }
        Ok(&url[id_start..id_end])
    }

    /// Get EpisodeInfo from url.
    pub async fn get_episode_info(url: String) -> Result<EpisodeInfo> {
        let (name, id) =
            tokio::try_join!(Self::parse_episode_name(&url), Self::parse_episode_id(&url))?;
        Ok(EpisodeInfo {
            name: name.to_owned(),
            id: id.to_owned(),
        })
    }

    /// Get a Vec of episode data urls from url.
    pub async fn get_show_episodes(&self, show_url: &str) -> Result<Vec<String>> {
        let url = Self::construct_show_query_url(show_url)?;
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

    /// Get data url for episode with id ep_id.
    #[async_recursion::async_recursion]
    pub async fn get_episode_url<'b>(&self, ep_id: &str) -> Result<String> {
        let url = Self::construct_ep_query_url(ep_id).await?;
        let token = self.token.lock().await;
        let result = self.net.get(url).bearer_auth(token).send().await?;

        let status = result.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            self.refresh_token().await?;
            return self.get_episode_url(ep_id).await;
        }
        if status != StatusCode::OK {
            return Err(format!("Status code was not 200 OK.\nCode: {}", status).into());
        }

        let text = result.text().await?;
        let json: Value = serde_json::from_str(&text)?;
        let root = json.get(0).ok_or_generic("Could not get JSON value.")?;
        let ep_url = root["url"]
            .as_str()
            .ok_or_generic("Could not get 'url' from root as str.")?;
        Ok(ep_url.to_owned())
    }
}
