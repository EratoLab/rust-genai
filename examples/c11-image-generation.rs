//! This example demonstrates how to generate images using the OpenAI DALL-E API

use genai::Client;
use genai::chat::{ContentPart, ImageRequest, ImageSource};

const MODEL: &str = "dall-e-3";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

	let client = Client::default();

	let prompt = "A serene landscape with mountains and a lake at sunset, digital art style";

	println!("\n--- Generating image with prompt:\n{prompt}");

	let image_req = ImageRequest::from_prompt(prompt)
		.with_size("1024x1024")
		.with_quality("standard")
		.with_style("vivid")
		.with_response_format("url");

	let image_res = client.exec_image_generation(MODEL, image_req, None).await?;

	println!("\n--- Generated {} image(s)", image_res.images.len());

	for (idx, image) in image_res.images.iter().enumerate() {
		match image {
			ContentPart::Image { content_type, source } => {
				match source {
					ImageSource::Url(url) => {
						println!("Image {}: {} (URL: {})", idx + 1, content_type, url);
					}
					ImageSource::Base64(_) => {
						println!("Image {}: {} (Base64 encoded)", idx + 1, content_type);
					}
				}
			}
			_ => println!("Unexpected content type"),
		}
	}

	Ok(())
}

