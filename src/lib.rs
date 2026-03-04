use nokhwa;

pub enum CameraType {
	Webcam,
	DSLR, // Not supported yet, but will be in the future.
}

pub struct PreacamCamera {
	pub id: String,
	pub name: String,
	pub camera_type: CameraType,
}

pub fn list_cameras() -> Vec<PreacamCamera> {
	let cameras_devices = nokhwa::query(nokhwa::utils::ApiBackend::Auto);
	let mut cameras = Vec::new();
	match cameras_devices {
		Ok(devices) => {
			for device in devices {
				cameras.push(PreacamCamera {
					id: device.index().to_string(),
					name: device.human_name(),
					camera_type: CameraType::Webcam,
				});
			}
		}
		Err(e) => {
			eprintln!("Error listing cameras: {}", e);
		}
	}
	cameras
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn list() {
		let cameras = list_cameras();
		assert!(!cameras.is_empty(), "No cameras found");
		for camera in cameras {
			println!("Camera ID: {}, Name: {}", camera.id, camera.name);
		}
	}

	#[test]
	fn start() {}
}
