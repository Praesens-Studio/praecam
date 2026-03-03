use nokhwa;

enum CameraType {
	Webcam,
	DSLR, // Not supported yet, but will be in the future.
}

struct Camera {
	id: String,
	name: String,
	camera_type: CameraType,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn list() {}

	#[test]
	fn start() {}
}
