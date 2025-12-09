//! Tests for OpenAI image generation

use genai::chat::{ContentPart, ImageRequest, ImageSource};
use genai::Client;

const MODEL: &str = "dall-e-3";

#[tokio::test]
async fn test_openai_image_generation_simple() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	let image_req = ImageRequest::from_prompt("A simple red circle on white background")
		.with_size("1024x1024")
		.with_quality("standard")
		.with_response_format("url");

	let image_res = client.exec_image_generation(MODEL, image_req, None).await?;

	// Verify we got at least one image
	assert!(!image_res.images.is_empty(), "Should generate at least one image");

	// Verify the first image is the correct type
	match &image_res.images[0] {
		ContentPart::Image { source, .. } => match source {
			ImageSource::Url(url) => {
				assert!(!url.is_empty(), "URL should not be empty");
				println!("Generated image URL: {}", url);
			}
			ImageSource::Base64(_) => {
				panic!("Expected URL format, got Base64");
			}
		},
		_ => panic!("Expected ContentPart::Image"),
	}

	Ok(())
}

#[tokio::test]
async fn test_openai_image_generation_multiple() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	let image_req = ImageRequest::from_prompt("A simple geometric shape")
		.with_n(2)
		.with_size("1024x1024")
		.with_quality("standard")
		.with_response_format("url");

	let image_res = client.exec_image_generation(MODEL, image_req, None).await?;

	// Verify we got the requested number of images
	assert_eq!(
		image_res.images.len(),
		2,
		"Should generate exactly 2 images"
	);

	Ok(())
}

#[tokio::test]
async fn test_openai_image_generation_b64() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	let image_req = ImageRequest::from_prompt("A simple blue square")
		.with_size("1024x1024")
		.with_quality("standard")
		.with_response_format("b64_json");

	let image_res = client.exec_image_generation(MODEL, image_req, None).await?;

	// Verify we got at least one image
	assert!(!image_res.images.is_empty(), "Should generate at least one image");

	// Verify the first image is base64 encoded
	match &image_res.images[0] {
		ContentPart::Image { source, .. } => match source {
			ImageSource::Base64(data) => {
				assert!(!data.is_empty(), "Base64 data should not be empty");
				println!("Generated image as base64, length: {}", data.len());
			}
			ImageSource::Url(_) => {
				panic!("Expected Base64 format, got URL");
			}
		},
		_ => panic!("Expected ContentPart::Image"),
	}

	Ok(())
}

#[tokio::test]
async fn test_unsupported_adapter_image_generation() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	let image_req = ImageRequest::from_prompt("A test image");

	// Try with Anthropic model which doesn't support image generation
	let result = client.exec_image_generation("claude-3-5-sonnet-20241022", image_req, None).await;

	assert!(result.is_err(), "Should return error for unsupported adapter");

	if let Err(e) = result {
		let error_msg = format!("{}", e);
		assert!(
			error_msg.contains("Service type") || error_msg.contains("not supported"),
			"Error should mention service type not supported"
		);
	}

	Ok(())
}

