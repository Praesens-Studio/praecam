use futures_util::SinkExt;
use gphoto2::Context;
use nokhwa::{
	Camera,
	pixel_format::RgbFormat,
	utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType},
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};

// Information returned by the `list()` command
#[derive(Debug, Clone)]
pub struct PreacamCameraInfo {
	pub id: String,
	pub name: String,
	pub description: String,
	pub source: String,
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

	let frame_interval_ms = {
		let fps = config.target_fps.max(1) as u64;
		1000 / fps
	};

	let capture_tx = tx.clone();
	let camera_index = config.camera_index;
	let capture_thread = std::thread::spawn(move || -> PraecamResult<()> {
		let requested =
			RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
		let mut camera = Camera::new(CameraIndex::Index(camera_index), requested)?;
		camera.open_stream()?;

		loop {
			let frame = camera.frame()?;
			let _ = capture_tx.send(frame.buffer().to_vec());
			std::thread::sleep(std::time::Duration::from_millis(frame_interval_ms));
		}
	});

	loop {
		let (stream, _) = listener.accept().await?;
		let rx = tx.subscribe();
		tokio::spawn(async move {
			if let Err(err) = handle_websocket_client(stream, rx).await {
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
) -> PraecamResult<()> {
	let mut websocket = accept_async(stream).await?;

	loop {
		let frame = rx.recv().await?;
		websocket.send(Message::Binary(frame)).await?;
	}
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
