use log::{info, warn};
use crate::cache::MapCache;
use std::env;
use std::net::{SocketAddr, IpAddr};
use std::str::FromStr;
use crate::request_manager::RequestManager;
use env_logger::Env;

mod models;
mod request_manager;
mod cache;
mod webserver;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::new().default_filter_or("info"));
    let version: Option<&str> = built_info::GIT_COMMIT_HASH;
    let dirty: Option<bool> = built_info::GIT_DIRTY;
    let profile: &str = built_info::PROFILE;
    let build_time: &str = built_info::BUILT_TIME_UTC;
    info!(
        "Starting aiosaber-beatsaver-cache with revision {} ({}), built with profile {} at {}",
        version.unwrap_or("{untagged build}"),
        if dirty.unwrap_or(true) {
            "dirty"
        } else {
            "clean"
        },
        profile,
        build_time
    );

    let port = env::var("SERVICE_PORT")
        .map(|str| str.parse().unwrap())
        .unwrap_or(2345u16);
    let host = env::var("SERVICE_HOST").unwrap_or("0.0.0.0".to_owned());
    let addr = SocketAddr::new(IpAddr::from_str(host.as_str()).unwrap(), port);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("AIOSaber-Backend - Map Meta Cache")
        .build()
        .unwrap();
    let (request_manager, ratelimit_handle) = RequestManager::new(client, 8, 9, std::time::Duration::from_millis(1050));
    let (cache, cache_handle) = MapCache::setup(request_manager);
    let web_handle = webserver::start_webserver(addr, cache);

    tokio::select! {
        _val = cache_handle => {
            warn!("Cache handle died, exiting");
        }
        _val = web_handle => {
            warn!("Web handle died, exiting");
        }
        _val = ratelimit_handle => {
            warn!("Ratelimit handle died, exiting");
        }
    }
}

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}