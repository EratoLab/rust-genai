//! This module contains all the types related to an Image Generation Request.

use serde::{Deserialize, Serialize};

// region:    --- ImageRequest

/// The Image Generation request for generating images from text prompts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageRequest {
	/// A text description of the desired image(s). The maximum length is 4000 characters.
	pub prompt: String,

	/// The number of images to generate. Must be between 1 and 10.
	pub n: Option<i32>,

	/// The size of the generated images. Must be one of "256x256", "512x512", "1024x1024", "1792x1024", or "1024x1792".
	pub size: Option<String>,

	/// The quality of the image that will be generated. "hd" creates images with finer details.
	/// Must be one of "standard" or "hd".
	pub quality: Option<String>,

	/// The style of the generated images. Must be one of "vivid" or "natural".
	pub style: Option<String>,

	/// The format in which the generated images are returned. Must be one of "url" or "b64_json".
	pub response_format: Option<String>,
}

/// Constructors
impl ImageRequest {
	/// Create a new ImageRequest with the given prompt.
	pub fn new(prompt: impl Into<String>) -> Self {
		Self {
			prompt: prompt.into(),
			n: None,
			size: None,
			quality: None,
			style: None,
			response_format: None,
		}
	}

	/// Create an ImageRequest from a prompt.
	pub fn from_prompt(prompt: impl Into<String>) -> Self {
		Self::new(prompt)
	}
}

/// Chainable Setters
impl ImageRequest {
	/// Set the number of images to generate.
	pub fn with_n(mut self, n: i32) -> Self {
		self.n = Some(n);
		self
	}

	/// Set the size of the generated images.
	pub fn with_size(mut self, size: impl Into<String>) -> Self {
		self.size = Some(size.into());
		self
	}

	/// Set the quality of the generated images.
	pub fn with_quality(mut self, quality: impl Into<String>) -> Self {
		self.quality = Some(quality.into());
		self
	}

	/// Set the style of the generated images.
	pub fn with_style(mut self, style: impl Into<String>) -> Self {
		self.style = Some(style.into());
		self
	}

	/// Set the response format for the generated images.
	pub fn with_response_format(mut self, response_format: impl Into<String>) -> Self {
		self.response_format = Some(response_format.into());
		self
	}
}

// endregion: --- ImageRequest

