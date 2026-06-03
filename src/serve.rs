pub fn serve(port: u16) -> anyhow::Result<()> {
    println!("Serving at http://localhost:{port}");

    tokio::runtime::Runtime::new()?.block_on(run_server(port));

    Ok(())
}

async fn run_server(port: u16) {
    use axum::Router;
    use tower_http::services::ServeDir;

    let app = Router::new()
        .fallback_service(ServeDir::new("dist").append_index_html_on_directories(true));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
