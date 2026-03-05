# Praesens Camera Library

## Streaming camera frames to WebSocket

The library now exposes:

- `list_cameras()` to discover camera IDs.
- `start_camera_websocket_stream(config)` to open a camera and broadcast frames to all websocket clients.

Example:

```rust
use praecam::{PraecamStreamConfig, start_camera_websocket_stream};

#[tokio::main]
async fn main() {
	let config = PraecamStreamConfig {
		camera_index: 0,
		websocket_bind_addr: "127.0.0.1:9001".to_string(),
		target_fps: 30,
		channel_capacity: 8,
	};

	start_camera_websocket_stream(config).await.unwrap();
}
```

Notes:

- The server sends each frame as a binary websocket message.
- Frame payloads are the raw bytes returned by `nokhwa`'s frame buffer.