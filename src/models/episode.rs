#[derive(Clone)]
pub struct EpisodeInfo {
    pub name: String,
    pub id: String,
}

#[derive(Clone)]
pub struct EpisodeData {
    pub info: EpisodeInfo,
    pub data: Vec<u8>,
}
