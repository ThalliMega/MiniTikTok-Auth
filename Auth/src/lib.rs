//! For integration tests.

use std::{env, error::Error, future::Future, net::Ipv6Addr, pin::Pin, time::Duration};

use auth_service::AuthService;
use bb8_postgres::{bb8, tokio_postgres::NoTls, PostgresConnectionManager};
use proto::auth_service_server::AuthServiceServer;
use tonic::{transport::Server, Response, Status};
use tonic_health::server::health_reporter;

mod auth_service;

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
pub async fn start_up() -> Result<(), DynError> {
    env_logger::init();

    let redis_client =
        redis::Client::open(env::var("REDIS_URL").map_err(|_| "REDIS_URL doesn't exist.")?)
            .map_err(|e| e.to_string())?;

    let postgres_config = env::var("POSTGRES_URL")
        .map_err(|_| "POSTGRES_URL doesn't exist.")?
        .parse()?;

    let postgres_manager = PostgresConnectionManager::new(postgres_config, NoTls);

    let (mut health_reporter, health_service) = health_reporter();

    let redis_conn = redis_client.get_multiplexed_tokio_connection().await?;

    let postgres_pool = bb8::Pool::builder().build(postgres_manager).await?;

    health_reporter
        .set_serving::<AuthServiceServer<AuthService>>()
        .await;

    Server::builder()
        .concurrency_limit_per_connection(256)
        .tcp_keepalive(Some(Duration::from_secs(10)))
        .add_service(AuthServiceServer::new(AuthService {
            redis_conn,
            postgres_pool,
        }))
        .add_service(health_service)
        .serve_with_shutdown(
            (Ipv6Addr::UNSPECIFIED, 14514).into(),
            // TODO?: unwrap
            async { tokio::signal::ctrl_c().await.unwrap() },
        )
        .await?;

    Ok(())
}

/// Build a runtime and block on a `Future`.
pub fn block_on<F: std::future::Future>(f: F) -> Result<F::Output, std::io::Error> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(f))
}
