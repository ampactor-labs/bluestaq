#[tokio::main]
async fn main() {
    let conn = match notes::db::new_file("notes.db") {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("failed to open notes.db: {e}");
            std::process::exit(1);
        }
    };
    let state = notes::AppState {
        db: std::sync::Arc::new(std::sync::Mutex::new(conn)),
    };

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
