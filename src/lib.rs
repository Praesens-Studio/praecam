use gphoto2::Context;

// Information returned by the `list()` command
#[derive(Debug, Clone)]
pub struct PreacamCameraInfo {
	pub id: String,
	pub name: String,
	pub description: String,
	pub source: String,
}

fn list_cameras() -> Result<Vec<PreacamCameraInfo>, Box<dyn std::error::Error>> {
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
	let nokhwa_camera_list = nokhwa::query(nokhwa::utils::ApiBackend::Auto)?;
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
