//! This module contains all the types related to an Image Generation Response.

use serde::{Deserialize, Serialize};

use crate::ModelIden;
use crate::chat::{ContentPart, Usage};

// region:    --- ImageResponse

/// The Image Generation response when performing an image generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
	/// The generated images as ContentPart::Image variants
	pub images: Vec<ContentPart>,

	/// The resolved Model Identifier (AdapterKind/ModelName) used for this request.
	pub model_iden: ModelIden,

	/// The eventual usage of the image generation response
	pub usage: Option<Usage>,

	/// The raw value of the response body, which can be used for provider specific features.
	pub captured_raw_body: Option<serde_json::Value>,
}

/// Getters
impl ImageResponse {
	/// Returns a reference to the first image if available.
	pub fn first_image(&self) -> Option<&ContentPart> {
		self.images.first()
	}

	/// Returns a vector of references to all images.
	pub fn all_images(&self) -> Vec<&ContentPart> {
		self.images.iter().collect()
	}

	/// Consumes the `ImageResponse` and returns all images.
	pub fn into_images(self) -> Vec<ContentPart> {
		self.images
	}
}

// endregion: --- ImageResponse

