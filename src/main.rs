#[tokio::main]
async fn main() {
    let state = notes::AppState::in_memory();

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:8080").await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("failed to bind 127.0.0.1:8080: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, notes::router(state)).await {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
