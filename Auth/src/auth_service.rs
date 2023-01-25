use crate::proto::{
    auth_response::AuthStatusCode, auth_service_server, token_response::TokenStatusCode,
    AuthRequest, AuthResponse, TokenRequest, TokenResponse,
};
use log::error;
use redis::{aio::MultiplexedConnection, AsyncCommands, Expiry};
use std::{future::Future, pin::Pin};
use tokio_postgres::NoTls;
use tonic::{Request, Response, Status};

type AsyncWrapper<'a, T> = Pin<Box<dyn Future<Output = Result<Response<T>, Status>> + Send + 'a>>;

pub struct AuthService {
    pub redis_conn: MultiplexedConnection,
    pub postgres_config: tokio_postgres::config::Config,
}

impl auth_service_server::AuthService for AuthService {
    fn auth<'life0, 'async_trait>(
        &'life0 self,
        request: Request<AuthRequest>,
    ) -> AsyncWrapper<'async_trait, AuthResponse>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let bad_database = Err(Status::internal("Bad Database"));

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
                        return bad_database;
                    }
                },
            ))
        })
    }

    fn retrive_token<'life0, 'async_trait>(
        &'life0 self,
        request: Request<TokenRequest>,
    ) -> AsyncWrapper<'async_trait, TokenResponse>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let bad_database = Err(Status::internal("Bad Database"));

        let req = request.into_inner();
        let username = req.username;
        let password = req.password;

        let fail_response = Ok(Response::new(TokenResponse {
            status_code: TokenStatusCode::Fail.into(),
            token: String::new(),
            user_id: 0,
        }));

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

            let (real_password, user_id): (String, u32) = match client
                .query_opt(
                    "SELECT password, id FROM auth WHERE username = $1",
                    &[&username],
                )
                .await
            {
                Ok(row) => {
                    if let Some(r) = row {
                        // TODO: into_err when stable
                        (
                            r.try_get(0).map_err(|e| {
                                error!("{e}");
                                unsafe { bad_database.as_ref().unwrap_err_unchecked().clone() }
                            })?,
                            r.try_get(1).map_err(|e| {
                                error!("{e}");
                                unsafe { bad_database.as_ref().unwrap_err_unchecked().clone() }
                            })?,
                        )
                    } else {
                        return fail_response;
                    }
                }
                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
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
                    user_id,
                })),

                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            }
        })
    }

    fn retrive_user_id<'life0, 'async_trait>(
        &'life0 self,
        request: tonic::Request<prost::alloc::string::String>,
    ) -> AsyncWrapper<'async_trait, u32>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let bad_database = Err(Status::internal("Bad Database"));

        Box::pin(async {
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

            match client
                .query_opt(
                    "SELECT id FROM auth WHERE username = $1",
                    &[&request.into_inner()],
                )
                .await
            {
                Ok(row) => {
                    if let Some(r) = row {
                        // TODO: into_err when stable
                        Ok(Response::new(r.try_get(0).map_err(|e| {
                            error!("{e}");
                            unsafe { bad_database.as_ref().unwrap_err_unchecked().clone() }
                        })?))
                    } else {
                        return Ok(Response::new(0));
                    }
                }
                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            }
        })
    }
}
