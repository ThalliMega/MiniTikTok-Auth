use crate::{
    proto::{
        auth_response::AuthStatusCode, auth_service_server, token_response::TokenStatusCode,
        AuthRequest, AuthResponse, TokenRequest, TokenResponse,
    },
    AsyncWrapper,
};
use argon2::{password_hash, Argon2, PasswordHash, PasswordVerifier};
use bb8_postgres::{bb8, tokio_postgres::NoTls, PostgresConnectionManager};
use log::{error, info};
use redis::{aio::MultiplexedConnection, AsyncCommands, Expiry};
use tonic::{Request, Response, Status};

pub struct AuthService {
    pub redis_conn: MultiplexedConnection,
    pub postgres_pool: bb8::Pool<PostgresConnectionManager<NoTls>>,
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
        let token = req.token;

        Box::pin(async move {
            Ok(Response::new(
                match self
                    .redis_conn
                    .clone()
                    .get_ex(&token, Expiry::EX(60 * 60 * 24 * 3))
                    .await
                {
                    Ok(Some(user_id)) => AuthResponse {
                        status_code: AuthStatusCode::Success.into(),
                        user_id,
                    },
                    Ok(None) => AuthResponse {
                        status_code: AuthStatusCode::Fail.into(),
                        user_id: 0,
                    },
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
        let bad_database_status = Status::internal("Bad Database");
        let bad_database = Err(bad_database_status.clone());

        let req = request.into_inner();
        let username = req.username;
        let password = req.password;

        let fail_response = Ok(Response::new(TokenResponse {
            status_code: TokenStatusCode::Fail.into(),
            token: String::new(),
            user_id: 0,
        }));

        Box::pin(async move {
            let client = match self.postgres_pool.get().await {
                Ok(val) => val,
                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            };

            let (password_hash, user_id): (String, i64) = match client
                .query_opt(
                    "SELECT password, id FROM auth WHERE username = $1",
                    &[&username],
                )
                .await
            {
                Ok(Some(row)) =>
                // TODO: into_err when stable
                {
                    (
                        row.try_get(0).map_err(|e| {
                            error!("{e}");
                            bad_database_status.clone()
                        })?,
                        row.try_get(1).map_err(|e| {
                            error!("{e}");
                            bad_database_status.clone()
                        })?,
                    )
                }
                Ok(None) => return fail_response,
                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            };

            let parsed_hash = PasswordHash::new(&password_hash).map_err(|e| {
                error!("parse password hash failed: {e}");
                bad_database_status
            })?;

            if let Err(e) = Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
                match e {
                    password_hash::Error::Password => {
                        info!("someone try to login {user_id} with wrong password")
                    }
                    _ => {
                        error!("hash verification failed: {e}")
                    }
                }
                return fail_response;
            }

            // TODO: Will uuid collide?
            let token = uuid::Uuid::new_v4().to_string();

            match self
                .redis_conn
                .clone()
                .set_ex(&token, &user_id, 60 * 60 * 24 * 3)
                .await
            {
                Ok(()) => Ok(Response::new(TokenResponse {
                    status_code: TokenStatusCode::Success.into(),
                    token,
                    user_id,
                })),

                Err(e) => {
                    error!("{e}");
                    bad_database
                }
            }
        })
    }
}
