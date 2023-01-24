use crate::proto::{
    auth_response::AuthStatusCode, auth_service_server, token_response::TokenStatusCode,
    AuthRequest, AuthResponse, TokenRequest, TokenResponse,
};
use log::error;
use redis::{aio::MultiplexedConnection, AsyncCommands, Expiry};
use std::{future::Future, pin::Pin};
use tokio_postgres::NoTls;
use tonic::{Request, Response, Status};

pub struct AuthService {
    pub redis_conn: MultiplexedConnection,
    pub postgres_config: tokio_postgres::config::Config,
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
                    .get_ex::<String, u32>(token, Expiry::EX(60 * 60 * 24 * 3))
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
                        error!("{e}");
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
        let req = request.into_inner();
        let username = req.username;
        let password = req.password;

        let fail_response = Ok(Response::new(TokenResponse {
            status_code: TokenStatusCode::Fail.into(),
            token: String::new(),
        }));

        let bad_database = Err(Status::internal("Bad Database"));

        Box::pin(async move {
            let (client, conn) = match self.postgres_config.connect(NoTls).await {
                Ok(val) => val,
                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            };

            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    error!("{e}");
                }
            });

            let rows = match client
                .query(
                    "SELECT password FROM auth WHERE username = $1",
                    &[&username],
                )
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            };

            let real_password: &str = if let Some(row) = rows.get(0) {
                row.get(0)
            } else {
                return fail_response;
            };

            if password != real_password {
                return fail_response;
            }

            // Will uuid collide?
            let token = uuid::Uuid::new_v4().to_string();

            match self
                .redis_conn
                .clone()
                .set_ex(&token, username, 60 * 60 * 24 * 3)
                .await
            {
                Ok(()) => Ok(Response::new(TokenResponse {
                    status_code: TokenStatusCode::Success.into(),
                    token,
                })),

                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            }
        })
    }
}
