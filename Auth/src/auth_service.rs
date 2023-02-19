use std::fmt::Display;

use crate::{
    proto::{
        auth_response::AuthStatusCode, auth_service_server, token_response::TokenStatusCode,
        AuthRequest, AuthResponse, TokenRequest, TokenResponse,
    },
    AsyncWrapper,
};
use argon2::{password_hash, Argon2, PasswordHash, PasswordVerifier};
use bb8_bolt::{bb8::Pool, bolt_client, bolt_proto};
use log::{error, info, warn};
use redis::{aio::MultiplexedConnection, AsyncCommands, Expiry};
use tonic::{Request, Response, Status};

pub struct AuthService {
    pub redis_conn: MultiplexedConnection,
    pub bolt_pool: Pool<bb8_bolt::Manager>,
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
            let mut client = map_bad_db_and_log(self.bolt_pool.get().await)?;

            transform_result(
                client
                    .run(
                        "MATCH (u:User { username: $username }) RETURN u.password_hash, u.id;",
                        Some([("username", username)].into_iter().collect()),
                        None,
                    )
                    .await,
            )?;

            let records =
                transform_records(client.pull(Some([("n", -1)].into_iter().collect())).await)?;

            let record = match records.get(0) {
                Some(r) => r.fields(),
                None => {
                    return Ok(Response::new(TokenResponse {
                        status_code: TokenStatusCode::Fail.into(),
                        ..Default::default()
                    }))
                }
            };

            let passhash = match record.get(0) {
                Some(bolt_proto::Value::String(s)) => s.as_str(),
                _ => return bad_database,
            };
            let user_id = match record.get(1) {
                Some(bolt_proto::Value::Integer(i)) => *i,
                _ => return bad_database,
            };

            let parsed_hash = PasswordHash::new(passhash).map_err(|e| {
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

fn transform_result(
    r: Result<bolt_proto::Message, bolt_client::error::CommunicationError>,
) -> Result<(), Status> {
    match r {
        Ok(bolt_proto::Message::Success(_)) => Ok(()),
        Ok(res) => {
            warn!("{res:?}");
            return Err(Status::internal("Bad Database"));
        }
        Err(e) => {
            error!("{e}");
            return Err(Status::internal("Bad Database"));
        }
    }
}

fn transform_records(
    r: Result<
        (Vec<bolt_proto::message::Record>, bolt_proto::Message),
        bolt_client::error::CommunicationError,
    >,
) -> Result<Vec<bolt_proto::message::Record>, Status> {
    match r {
        Ok((rec, bolt_proto::Message::Success(_))) => Ok(rec),
        Ok((_, res)) => {
            warn!("{res:?}");
            return Err(Status::internal("Bad Database"));
        }
        Err(e) => {
            error!("{e}");
            return Err(Status::internal("Bad Database"));
        }
    }
}

fn map_bad_db_and_log<O, E: Display>(res: Result<O, E>) -> Result<O, Status> {
    res.map_err(|e| {
        error!("{e}");
        Status::internal("Bad Database")
    })
}
