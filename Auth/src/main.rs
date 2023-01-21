mod combind_incoming;
mod proto;

use std::env;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::pin::Pin;
use std::time::Duration;

use combind_incoming::CombinedIncoming;
use log::warn;
use proto::auth_response::AuthStatusCode;
use proto::auth_service_server::{self, AuthServiceServer};
use proto::{AuthRequest, AuthResponse, TokenRequest, TokenResponse};
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Expiry};
use std::future::Future;
use tonic::transport::{server::TcpIncoming, Server};
use tonic::{Request, Response, Status};

struct AuthService {
    redis_conn: MultiplexedConnection,
}

impl auth_service_server::AuthService for AuthService {
    fn auth<'life0, 'async_trait>(
        &'life0 self,
        request: Request<AuthRequest>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<AuthResponse>, Status>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let req = request.into_inner();
        let user_id = req.user_id;
        let token = req.token;

        Box::pin(async move {
            Ok(Response::new(
                match self
                    .redis_conn
                    .clone()
                    .get_ex::<String, String>(token, Expiry::EX(60 * 60 * 24 * 3))
                    .await
                {
                    Ok(id) => {
                        if id == user_id {
                            AuthResponse {
                                status_code: AuthStatusCode::Success.into(),
                            }
                        } else {
                            AuthResponse {
                                status_code: AuthStatusCode::Fail.into(),
                            }
                        }
                    }

                    Err(e) => {
                        warn!("{e}");
                        return Err(Status::internal("Bad Database"));
                    }
                },
            ))
        })
    }

    fn retrive_token<'life0, 'async_trait>(
        &'life0 self,
        request: Request<TokenRequest>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<TokenResponse>, Status>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }
}

fn main() {
    let redis_client = redis::Client::open(env::var("REDIS_URL").unwrap()).unwrap();

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
                .add_service(AuthServiceServer::new(AuthService { redis_conn }))
                .serve_with_incoming(CombinedIncoming {
                    a: TcpIncoming::new((Ipv6Addr::UNSPECIFIED, 14514).into(), false, None)
                        .unwrap(),
                    b: TcpIncoming::new((Ipv4Addr::UNSPECIFIED, 14514).into(), false, None)
                        .unwrap(),
                })
                .await
                .unwrap();
        })
}
