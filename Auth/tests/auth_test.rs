use mini_tiktok_auth::{
    proto::{
        auth_response::AuthStatusCode, auth_service_client::AuthServiceClient, AuthRequest,
        AuthResponse,
    },
    start_up,
};

#[test]
fn auth() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let handle = start_up().unwrap();

            let mut channel = AuthServiceClient::connect("http://localhost:14514")
                .await
                .unwrap();

            assert_eq!(
                channel
                    .auth(AuthRequest {
                        user_id: "114514".into(),
                        token: "1919810".into(),
                    })
                    .await
                    .unwrap()
                    .into_inner(),
                AuthResponse {
                    status_code: AuthStatusCode::Success.into()
                }
            );

            drop(handle)
        })
}
