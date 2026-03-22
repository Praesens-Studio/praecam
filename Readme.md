# one23 Shot Studio Camera Library

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
- Binary payload format is:
	- Bytes `0..4`: frame width (`u32`, little-endian)
	- Bytes `4..8`: frame height (`u32`, little-endian)
	- Bytes `8..`: RGB data (`width * height * 3` bytes)

### WebSocket commands

Send JSON text messages on the same websocket connection:

- List cameras:

```json
{"cmd":"list"}
```

- Switch active camera (nokhwa index):

```json
{"cmd":"switch","camera_index":1}
```

Server JSON responses:

- `{"type":"camera_list","active_camera_index":0,"cameras":[...]}`
- `{"type":"camera_switched","camera_index":1}`
- `{"type":"error","message":"..."}`

## Browser viewer example

A minimal browser viewer is available at `examples/ws_viewer.html`.

Open it directly in a browser, click **Connect**, and it will render frames from `ws://127.0.0.1:9001`.

## Hosted client (uses this library)

This crate now includes a runnable binary client in `src/main.rs`.

Run:

```bash
cargo run
```

It will:

- Start camera websocket streaming via `start_camera_websocket_stream(...)`
- Host the HTML page at `http://127.0.0.1:8080`


## Native dependencies required

This project requires the following system libraries to be installed at runtime:

- **Linux:** `libgphoto2` (install via your package manager, e.g. `sudo apt install libgphoto2-6` or `sudo dnf install libgphoto2`)
- **macOS:** `libgphoto2` (install via Homebrew: `brew install libgphoto2`)
- **Windows (MSYS2/MinGW):** `mingw-w64-x86_64-libgphoto2` (install via MSYS2: `pacman -S mingw-w64-x86_64-libgphoto2`)

These must be present on the user's system to run binaries built from this crate.

Optional environment variables:

- `PRAECAM_CAMERA_INDEX` (default: `0`)
- `PRAECAM_WS_BIND` (default: `127.0.0.1:9001`)
- `PRAECAM_HTTP_BIND` (default: `127.0.0.1:8080`)