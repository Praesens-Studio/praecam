use nokhwa::{NokhwaError, query, utils::CameraInfo};

pub fn list_camera_devices() -> Result<Vec<CameraInfo>, NokhwaError> {
	query(nokhwa::utils::ApiBackend::Auto)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn list() {
		let devices = list_camera_devices();
		match devices {
			Ok(devs) => {
				println!("Found {} camera(s):", devs.len());
				for (i, dev) in devs.iter().enumerate() {
					println!("{}: {}", i + 1, dev);
				}
			}
			Err(e) => {
				eprintln!("Error listing camera devices: {}", e);
			}
		}
	}
}
