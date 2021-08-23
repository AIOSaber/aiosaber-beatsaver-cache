use std::sync::Arc;
use cache_loader_async::cache_api::{LoadingCache, CacheLoadingError};
use cache_loader_async::backing::TtlCacheBacking;
use tokio::task::JoinHandle;
use log::warn;
use crate::models::BeatSaverMap;
use crate::request_manager::RequestManager;

pub type LoaderError = crate::request_manager::RequestError;

#[derive(Clone)]
pub struct MapCache {
    hash: LoadingCache<String, Arc<BeatSaverMap>, Arc<LoaderError>>,
    id: LoadingCache<u32, Arc<BeatSaverMap>, Arc<LoaderError>>,
}

impl MapCache {
    pub fn setup(request_manager: RequestManager) -> (MapCache, JoinHandle<()>) {
        let outer_manager = request_manager.clone();
        let (loading_cache_by_hash, hash_handle) = LoadingCache::with_backing(
            TtlCacheBacking::new(std::time::Duration::from_secs(86400)),
            move |map_hash: String| {
                let request_manager = outer_manager.clone();
                let mut request_url = "https://api.beatsaver.com/maps/hash/".to_owned();
                request_url.push_str(map_hash.as_str());
                async move {
                    let result = request_manager.queue_request(|client| client.get(request_url).build()).await;
                    response_to_map(result).await
                }
            });

        let (loading_cache_by_key, key_handle) = LoadingCache::with_backing(
            TtlCacheBacking::new(std::time::Duration::from_secs(86400)),
            move |map_key: u32| {
                let request_manager = request_manager.clone();
                let mut request_url = "https://api.beatsaver.com/maps/id/".to_owned();
                request_url.push_str(format!("{:x}", map_key).as_str());
                async move {
                    let result = request_manager.queue_request(|client| client.get(request_url).build()).await;
                    response_to_map(result).await
                }
            });

        let keep_alive_handle = tokio::spawn(async move {
            tokio::select! {
                _val = hash_handle => {
                    warn!("LoadingCache 'by-hash' died.");
                }
                _val = key_handle => {
                    warn!("LoadingCache 'by-key' died.");
                }
            }
        });

        (MapCache {
            hash: loading_cache_by_hash,
            id: loading_cache_by_key,
        }, keep_alive_handle)
    }

    pub async fn get_map_by_id(&self, map_id: u32) -> Result<Arc<BeatSaverMap>, CacheLoadingError<Arc<LoaderError>>> {
        let result = self.id.get_with_meta(map_id).await;
        if let Ok(meta) = result {
            if meta.cached {
                Ok(meta.result)
            } else {
                self.insert_map_by_id(meta.result.clone()).await;
                Ok(meta.result)
            }
        } else {
            result.map(|meta| meta.result)
        }
    }

    pub async fn get_map_by_hash(&self, map_hash: String) -> Result<Arc<BeatSaverMap>, CacheLoadingError<Arc<LoaderError>>> {
        let result = self.hash.get_with_meta(map_hash).await;
        if let Ok(meta) = result {
            if meta.cached {
                Ok(meta.result)
            } else {
                self.insert_map_by_hash(meta.result.clone()).await;
                Ok(meta.result)
            }
        } else {
            result.map(|meta| meta.result)
        }
    }

    // Since the 'by-key' LoadingCache was loading the key, it is already updated, so we only
    // need to update the 'by-hash' LoadingCache, to keep them in sync.
    async fn insert_map_by_id(&self, map: Arc<BeatSaverMap>) {
        for version in map.versions.iter() {
            self.hash.set(version.hash.clone(), map.clone()).await.ok();
        }
    }

    async fn insert_map_by_hash(&self, map: Arc<BeatSaverMap>) {
        self.insert_map_by_id(map.clone()).await; // in case the map has multiple versions we trigger another update
        self.id.set(u32::from_str_radix(map.id.as_str(), 16).unwrap_or_default(), map).await.ok();
    }
}

async fn response_to_map(response: Result<reqwest::Response, LoaderError>) -> Result<Arc<BeatSaverMap>, Arc<LoaderError>> {
    match response {
        Ok(response) => {
            response.json().await
                .map(|map| Arc::new(map))
                .map_err(|json_err| Arc::new(LoaderError::JsonError(json_err)))
        }
        Err(err) => Err(Arc::new(err))
    }
}