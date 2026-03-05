use futures_util::{SinkExt, StreamExt};
use gphoto2::Context;
use nokhwa::{
	Camera,
	pixel_format::RgbFormat,
	utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{
	Arc,
	atomic::{AtomicU32, Ordering},
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};

// Information returned by the `list()` command
#[derive(Debug, Clone, Serialize)]
pub struct PreacamCameraInfo {
	pub id: String,
	pub name: String,
	pub description: String,
	pub source: String,
}

#[derive(Debug, Deserialize)]
struct WsCommand {
	cmd: String,
	camera_index: Option<u32>,
}

pub fn list_cameras() -> Result<Vec<PreacamCameraInfo>, Box<dyn std::error::Error>> {
	let mut cameras = Vec::<PreacamCameraInfo>::new();

	// Get cameras from gphoto2
	let context = Context::new()?;
	let gphoto2_camera_list = context.list_cameras().wait()?;
	for camera in gphoto2_camera_list {
		cameras.push(PreacamCameraInfo {
			id: camera.port.clone(),
			name: camera.model.clone(),
			description: format!(
				"Camera model {} on port {}",
				camera.model,
				camera.port.clone()
			),
			source: "gphoto2".into(),
		});
	}

	// Get cameras from nokhwa
	let nokhwa_camera_list = nokhwa::query(ApiBackend::Auto)?;
	for (index, camera) in nokhwa_camera_list.into_iter().enumerate() {
		cameras.push(PreacamCameraInfo {
			id: index.to_string(),
			name: camera.human_name(),
			description: format!("Nokhwa camera at index {}", camera.index()),
			source: "nokhwa".into(),
		});
	}

	Ok(cameras)
}

pub type PraecamResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct PraecamStreamConfig {
	pub camera_index: u32,
	pub websocket_bind_addr: String,
	pub target_fps: u32,
	pub channel_capacity: usize,
}

fn open_nokhwa_camera(camera_index: u32) -> PraecamResult<Camera> {
	let requested =
		RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
	let mut camera = Camera::new(CameraIndex::Index(camera_index), requested)?;
	camera.open_stream()?;
	Ok(camera)
}

impl Default for PraecamStreamConfig {
	fn default() -> Self {
		Self {
			camera_index: 0,
			websocket_bind_addr: "127.0.0.1:9001".to_string(),
			target_fps: 30,
			channel_capacity: 8,
		}
	}
}

pub async fn start_camera_websocket_stream(config: PraecamStreamConfig) -> PraecamResult<()> {
	let listener = TcpListener::bind(&config.websocket_bind_addr).await?;
	let (tx, _) = broadcast::channel::<Vec<u8>>(config.channel_capacity);
	let selected_camera_index = Arc::new(AtomicU32::new(config.camera_index));

	let frame_interval_ms = {
		let fps = config.target_fps.max(1) as u64;
		1000 / fps
	};

	let capture_tx = tx.clone();
	let selected_camera_index_for_capture = selected_camera_index.clone();
	let capture_thread = std::thread::spawn(move || -> PraecamResult<()> {
		let mut active_camera_index: Option<u32> = None;
		let mut active_camera: Option<Camera> = None;

		loop {
			let requested_index = selected_camera_index_for_capture.load(Ordering::Relaxed);

			if active_camera.is_none() || active_camera_index != Some(requested_index) {
				match open_nokhwa_camera(requested_index) {
					Ok(camera) => {
						active_camera = Some(camera);
						active_camera_index = Some(requested_index);
					}
					Err(err) => {
						eprintln!("failed to initialize camera {}: {err}", requested_index);

						if let Some(current_index) = active_camera_index {
							selected_camera_index_for_capture
								.store(current_index, Ordering::Relaxed);
						} else {
							std::thread::sleep(std::time::Duration::from_millis(500));
							continue;
						}
					}
				}
			}

			let Some(camera) = active_camera.as_mut() else {
				std::thread::sleep(std::time::Duration::from_millis(100));
				continue;
			};

			let frame = match camera.frame() {
				Ok(frame) => frame,
				Err(err) => {
					eprintln!("camera frame error: {err}");
					active_camera = None;
					active_camera_index = None;
					std::thread::sleep(std::time::Duration::from_millis(100));
					continue;
				}
			};
			let decoded = match frame.decode_image::<RgbFormat>() {
				Ok(decoded) => decoded,
				Err(err) => {
					eprintln!("failed to decode frame to RGB: {err}");
					std::thread::sleep(std::time::Duration::from_millis(30));
					continue;
				}
			};

			let width = decoded.width();
			let height = decoded.height();
			let rgb = decoded.into_raw();

			let mut payload = Vec::with_capacity(8 + rgb.len());
			payload.extend_from_slice(&width.to_le_bytes());
			payload.extend_from_slice(&height.to_le_bytes());
			payload.extend_from_slice(&rgb);

			let _ = capture_tx.send(payload);
			std::thread::sleep(std::time::Duration::from_millis(frame_interval_ms));
		}
	});

	loop {
		let (stream, _) = listener.accept().await?;
		let rx = tx.subscribe();
		let selected_camera_index_for_client = selected_camera_index.clone();
		tokio::spawn(async move {
			if let Err(err) =
				handle_websocket_client(stream, rx, selected_camera_index_for_client).await
			{
				eprintln!("websocket client error: {err}");
			}
		});

		if capture_thread.is_finished() {
			break;
		}
	}

	Ok(())
}

async fn handle_websocket_client(
	stream: TcpStream,
	mut rx: broadcast::Receiver<Vec<u8>>,
	selected_camera_index: Arc<AtomicU32>,
) -> PraecamResult<()> {
	let websocket = accept_async(stream).await?;
	let (mut writer, mut reader) = websocket.split();

	loop {
		tokio::select! {
			frame_result = rx.recv() => {
				match frame_result {
					Ok(frame) => {
						if writer.send(Message::Binary(frame.into())).await.is_err() {
							return Ok(());
						}
					}
					Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
					Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
				}
			}
			incoming = reader.next() => {
				match incoming {
					Some(Ok(Message::Text(text))) => {
						handle_ws_command(&mut writer, &text, &selected_camera_index).await?;
					}
					Some(Ok(Message::Ping(payload))) => {
						if writer.send(Message::Pong(payload)).await.is_err() {
							return Ok(());
						}
					}
					Some(Ok(Message::Close(_))) => return Ok(()),
					Some(Ok(_)) => {}
					Some(Err(_)) => return Ok(()),
					None => return Ok(()),
				}
			}
		}
	}
}

async fn handle_ws_command<S>(
	writer: &mut S,
	text: &str,
	selected_camera_index: &Arc<AtomicU32>,
) -> PraecamResult<()>
where
	S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
	let command = match serde_json::from_str::<WsCommand>(text) {
		Ok(command) => command,
		Err(err) => {
			let response = json!({
				"type": "error",
				"message": format!("invalid command JSON: {err}"),
			});
			let _ = writer
				.send(Message::Text(response.to_string().into()))
				.await;
			return Ok(());
		}
	};

	match command.cmd.as_str() {
		"list" => {
			let response = match list_cameras() {
				Ok(cameras) => json!({
					"type": "camera_list",
					"active_camera_index": selected_camera_index.load(Ordering::Relaxed),
					"cameras": cameras,
				}),
				Err(err) => json!({
					"type": "error",
					"message": format!("failed to list cameras: {err}"),
				}),
			};

			let _ = writer
				.send(Message::Text(response.to_string().into()))
				.await;
		}
		"switch" => {
			let Some(camera_index) = command.camera_index else {
				let response = json!({
					"type": "error",
					"message": "missing camera_index",
				});
				let _ = writer
					.send(Message::Text(response.to_string().into()))
					.await;
				return Ok(());
			};

			let switch_result = open_nokhwa_camera(camera_index)
				.map(|_| ())
				.map_err(|err| err.to_string());

			match switch_result {
				Ok(_) => {
					selected_camera_index.store(camera_index, Ordering::Relaxed);

					let response = json!({
						"type": "camera_switched",
						"camera_index": camera_index,
					});
					let _ = writer
						.send(Message::Text(response.to_string().into()))
						.await;
				}
				Err(err) => {
					let active_index = selected_camera_index.load(Ordering::Relaxed);
					let response = json!({
						"type": "error",
						"message": format!("failed to switch camera to {}: {}", camera_index, err),
						"active_camera_index": active_index,
					});
					let _ = writer
						.send(Message::Text(response.to_string().into()))
						.await;
				}
			}
		}
		_ => {
			let response = json!({
				"type": "error",
				"message": "unknown command. supported: list, switch",
			});
			let _ = writer
				.send(Message::Text(response.to_string().into()))
				.await;
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn list() {
		let cameras = list_cameras().unwrap();
		println!("Found {} cameras", cameras.len());
		for camera in cameras {
			println!(
				"Camera: {} - {} ({}) [{}]",
				camera.id, camera.name, camera.description, camera.source
			);
		}
	}

	#[test]
	fn start() {}
}
