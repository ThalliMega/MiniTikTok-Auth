use std::{
    env,
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use auth_service::AuthService;
use combind_incoming::CombinedIncoming;
use proto::auth_service_server::AuthServiceServer;
use tonic::transport::Server;

mod auth_service;
mod combind_incoming;
mod proto;

fn main() {
    env_logger::init();

    let redis_client = redis::Client::open(env::var("REDIS_URL").unwrap()).unwrap();

    let mut postgres_config = tokio_postgres::config::Config::new();
    postgres_config.options(&env::var("POSTGRES_URL").unwrap());

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let redis_conn = redis_client
                .get_multiplexed_tokio_connection()
                .await
                .unwrap();

            Server::builder()
                .concurrency_limit_per_connection(256)
                .tcp_keepalive(Some(Duration::from_secs(10)))
                .add_service(AuthServiceServer::new(AuthService {
                    redis_conn,
                    postgres_config,
                }))
                .serve_with_incoming(
                    CombinedIncoming::new(
                        (Ipv6Addr::UNSPECIFIED, 14514).into(),
                        (Ipv4Addr::UNSPECIFIED, 14514).into(),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
        })
}
