//! For integration tests.

use std::{
    env,
    error::Error,
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use auth_service::AuthService;
use combind_incoming::CombinedIncoming;
use proto::auth_service_server::AuthServiceServer;
use tokio::task::JoinHandle;
use tonic::transport::Server;

mod auth_service;
mod combind_incoming;
pub mod proto;

type DynError = Box<dyn Error + Send + Sync>;

/// This function will initialize the [env-logger](https://docs.rs/env_logger) and start the server.  
/// Because this function will be used in integration tests,
/// it will **NOT** block the main thread.
pub fn start_up() -> Result<JoinHandle<Result<(), DynError>>, DynError> {
    env_logger::init();

    let redis_client = redis::Client::open(env::var("REDIS_URL")?)?;

    let mut postgres_config = tokio_postgres::config::Config::new();
    postgres_config.options(&env::var("POSTGRES_URL")?);

    Ok(tokio::spawn(async move {
        let redis_conn = redis_client.get_multiplexed_tokio_connection().await?;

        Server::builder()
            .concurrency_limit_per_connection(256)
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .add_service(AuthServiceServer::new(AuthService {
                redis_conn,
                postgres_config,
            }))
            .serve_with_incoming(CombinedIncoming::new(
                (Ipv6Addr::UNSPECIFIED, 14514).into(),
                (Ipv4Addr::UNSPECIFIED, 14514).into(),
            )?)
            .await?;

        Ok::<(), Box<dyn Error + Send + Sync>>(())
    }))
}
