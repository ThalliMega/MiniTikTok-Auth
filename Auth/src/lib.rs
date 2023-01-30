//! For integration tests.

use std::{
    env,
    error::Error,
    future::Future,
    net::{Ipv4Addr, Ipv6Addr},
    pin::Pin,
    time::Duration,
};

use auth_service::AuthService;
use bb8_postgres::{
    bb8,
    tokio_postgres::{self, NoTls},
    PostgresConnectionManager,
};
use combind_incoming::CombinedIncoming;
use health_check::HealthChecker;
use proto::{auth_service_server::AuthServiceServer, health_checker_server::HealthCheckerServer};
use tokio::task::JoinHandle;
use tonic::{transport::Server, Response, Status};

mod auth_service;
mod combind_incoming;
mod health_check;

pub mod proto;

type DynError = Box<dyn Error + Send + Sync>;

type AsyncWrapper<'a, T> = Pin<Box<dyn Future<Output = Result<Response<T>, Status>> + Send + 'a>>;

/// This function will initialize the [env-logger](https://docs.rs/env_logger) and start the server.  
/// Because this function will be used in integration tests,
/// it will **NOT** block the main thread.
///
/// # Panics
///
/// Panics if called from **outside** of the Tokio runtime.
pub fn start_up() -> Result<JoinHandle<Result<(), DynError>>, String> {
    env_logger::init();

    let redis_client = redis::Client::open(
        env::var("REDIS_URL").map_err(|_| "REDIS_URL doesn't exist.".to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let mut postgres_config = tokio_postgres::config::Config::new();
    postgres_config
        .options(&env::var("POSTGRES_URL").map_err(|_| "POSTGRES_URL doesn't exist.".to_string())?);

    let postgres_manager = PostgresConnectionManager::new(postgres_config, NoTls);

    Ok(tokio::spawn(async move {
        let redis_conn = redis_client.get_multiplexed_tokio_connection().await?;

        let postgres_pool = bb8::Pool::builder().build(postgres_manager).await?;

        Server::builder()
            .concurrency_limit_per_connection(256)
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .add_service(AuthServiceServer::new(AuthService {
                redis_conn,
                postgres_pool,
            }))
            .add_service(HealthCheckerServer::new(HealthChecker))
            .serve_with_incoming(CombinedIncoming::new(
                (Ipv6Addr::UNSPECIFIED, 14514).into(),
                (Ipv4Addr::UNSPECIFIED, 14514).into(),
            )?)
            .await?;

        Ok(())
    }))
}

/// Build a runtime and block on a `Future`.
pub fn block_on<F: std::future::Future>(f: F) -> Result<F::Output, std::io::Error> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(f))
}
