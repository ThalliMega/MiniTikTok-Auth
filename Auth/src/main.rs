use mini_tiktok_auth::start_up;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(start_up().unwrap())
        .unwrap()
        .unwrap()
}
