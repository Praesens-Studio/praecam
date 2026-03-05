use axum::{Router, response::Html, routing::get};
use praecam::{PraecamStreamConfig, start_camera_websocket_stream};
use tokio::net::TcpListener;

const VIEWER_HTML: &str = include_str!("../examples/ws_viewer.html");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let ws_bind = std::env::var("PRAECAM_WS_BIND").unwrap_or_else(|_| "127.0.0.1:9001".to_string());
	let http_bind =
		std::env::var("PRAECAM_HTTP_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
	let camera_index = std::env::var("PRAECAM_CAMERA_INDEX")
		.ok()
		.and_then(|v| v.parse::<u32>().ok())
		.unwrap_or(0);

	let ws_config = PraecamStreamConfig {
		camera_index,
		websocket_bind_addr: ws_bind.clone(),
		target_fps: 30,
		channel_capacity: 8,
	};

	let ws_task = tokio::spawn(async move {
		if let Err(err) = start_camera_websocket_stream(ws_config).await {
			eprintln!("websocket stream server error: {err}");
		}
	});

	let app = Router::new().route("/", get(|| async { Html(VIEWER_HTML) }));
	let listener = TcpListener::bind(&http_bind).await?;

	println!("Praecam camera ws server: ws://{}", ws_bind);
	println!("Praecam HTML viewer: http://{}", http_bind);

	axum::serve(listener, app).await?;
	ws_task.abort();
	Ok(())
}
