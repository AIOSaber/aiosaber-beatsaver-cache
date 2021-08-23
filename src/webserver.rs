use tokio::task::JoinHandle;
use std::net::SocketAddr;
use crate::cache::{MapCache, LoaderError};
use warp::{Filter, Reply};
use std::sync::Arc;
use crate::request_manager::RequestError;
use crate::models::BeatSaverMap;
use cache_loader_async::cache_api::CacheLoadingError;
use warp::http::StatusCode;
use log::error;

pub(crate) fn start_webserver(addr: SocketAddr, cache: MapCache) -> JoinHandle<()> {
    tokio::spawn(async move {
        let options = warp::options().map(|| Box::new("OK"));

        let inner_cache = cache.clone();
        let by_hash = warp::path!("maps" / "hash" / String)
            .and(warp::get())
            .and(warp::any().map(move || inner_cache.clone()))
            .and_then(by_hash);
        let inner_cache = cache.clone();
        let by_id = warp::path!("maps" / "id" / String)
            .and(warp::get())
            .and(warp::any().map(move || inner_cache.clone()))
            .and_then(by_id);

        warp::serve(options
            .or(by_hash)
            .or(by_id))
            .run(addr)
            .await
    })
}

async fn by_hash(hash: String, cache: MapCache) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    handle_result(hash.clone().as_str(), cache.get_map_by_hash(hash).await)
}

async fn by_id(id: String, cache: MapCache) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    match u32::from_str_radix(id.as_str(), 16) {
        Ok(key) => {
            handle_result(id.as_str(), cache.get_map_by_id(key).await)
        }
        Err(_) => {
            Ok(Box::new(warp::reply::with_status("Key is not a hex-formatted number", StatusCode::BAD_REQUEST)))
        }
    }
}

fn handle_result(key: &str, result: Result<Arc<BeatSaverMap>, CacheLoadingError<Arc<LoaderError>>>) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    match result {
        Ok(map) => {
            Ok(Box::new(warp::reply::json(map.as_ref())))
        }
        Err(err) => {
            match err {
                CacheLoadingError::CommunicationError(err) => {
                    error!("{}: Cache Communication Error: {:?}", key, err);
                    internal_server_error("An internal error occurred when communicating with the cache")
                }
                CacheLoadingError::NoData() => {
                    error!("{}: Cache returned no data", key);
                    internal_server_error("The internal cache returned no data")
                }
                CacheLoadingError::LoadingError(err) => {
                    error!("{}: An unexpected error occurred when loading the data: {:?}", key, err.as_ref());
                    match err.as_ref() {
                        RequestError::StatusCodeError(status) => status_code_error("Upstream request didn't return successfully", status),
                        RequestError::ReqwestError(err) => internal_server_error(format!("Upstream request failed: {:?}", err)),
                        RequestError::BufferError() => internal_server_error("Buffer is full"),
                        RequestError::JsonError(err) => internal_server_error(format!("Upstream returned invalid json - {:?}", err)),
                        _ => internal_server_error("An internal error occurred in the loader")
                    }
                }
            }
        }
    }
}

fn internal_server_error<T: Reply + 'static>(message: T) -> Result<Box<dyn Reply>, warp::Rejection> {
    Ok(Box::new(warp::reply::with_status(message, StatusCode::INTERNAL_SERVER_ERROR)))
}

fn status_code_error<T: Reply + 'static>(message: T, status: &u16) -> Result<Box<dyn Reply>, warp::Rejection> {
    Ok(Box::new(warp::reply::with_status(message, StatusCode::from_u16(status.clone()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))))
}