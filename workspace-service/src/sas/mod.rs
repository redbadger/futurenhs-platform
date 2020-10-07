pub use upload::create_upload_sas;
use url::Url;

mod upload;

#[derive(Debug, Clone)]
pub struct Config {
    pub access_key: String,
    pub upload_container_url: Url,
    pub files_container_url: Url,
}

impl Config {
    pub fn new(access_key: String, upload_container_url: Url, files_container_url: Url) -> Self {
        Self {
            access_key,
            upload_container_url,
            files_container_url,
        }
    }
}
