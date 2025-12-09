# Image Generation Support

This document describes the image generation functionality added to the rust-genai library.

## Overview

The library now supports image generation via the OpenAI DALL-E API and its Azure OpenAI equivalent. The implementation follows the same architectural patterns as the existing chat functionality.

## Features

- Generate images from text prompts using OpenAI's DALL-E models
- Support for both OpenAI and Azure OpenAI endpoints
- Configurable image parameters (size, quality, style, response format)
- Generate single or multiple images in one request
- Support for both URL and base64-encoded image responses
- Proper error handling for adapters that don't support image generation

## API Usage

### Basic Example

```rust
use genai::Client;
use genai::chat::ImageRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::default();
    
    let image_req = ImageRequest::from_prompt("A serene landscape with mountains")
        .with_size("1024x1024")
        .with_quality("standard")
        .with_style("vivid");
    
    let image_res = client.exec_image_generation("dall-e-3", image_req, None).await?;
    
    println!("Generated {} image(s)", image_res.images.len());
    Ok(())
}
```

### ImageRequest Parameters

- `prompt` (required): Text description of the desired image (max 4000 characters)
- `n` (optional): Number of images to generate (1-10)
- `size` (optional): Image dimensions ("256x256", "512x512", "1024x1024", "1792x1024", "1024x1792")
- `quality` (optional): Image quality ("standard" or "hd")
- `style` (optional): Image style ("vivid" or "natural")
- `response_format` (optional): Response format ("url" or "b64_json")

### ImageResponse

The response contains:
- `images`: Vector of `ContentPart::Image` containing the generated images
- `model_iden`: The model identifier used for generation
- `usage`: Optional usage statistics
- `captured_raw_body`: Optional raw response body (if capture is enabled)

### Accessing Generated Images

```rust
for image in image_res.images.iter() {
    match image {
        ContentPart::Image { content_type, source } => {
            match source {
                ImageSource::Url(url) => {
                    println!("Image URL: {}", url);
                }
                ImageSource::Base64(data) => {
                    println!("Base64 data length: {}", data.len());
                }
            }
        }
        _ => {}
    }
}
```

## Azure OpenAI Support

The implementation preserves query parameters and headers in the URL, which allows it to work with Azure OpenAI endpoints. Configure your client with the appropriate endpoint and authentication:

```rust
use genai::Client;
use genai::resolver::{AuthData, Endpoint};

let client = Client::builder()
    .with_auth_resolver(/* your Azure auth config */)
    .build();

// Use with Azure endpoint
let image_res = client.exec_image_generation("dall-e-3", image_req, None).await?;
```

## Supported Models

- `dall-e-3`: OpenAI's latest DALL-E model
- `dall-e-2`: Previous generation DALL-E model
- Azure OpenAI deployments of DALL-E models

## Error Handling

Attempting to generate images with adapters that don't support image generation (Anthropic, Cohere, Gemini, etc.) will return a `ServiceTypeNotSupported` error:

```rust
let result = client.exec_image_generation("claude-3-5-sonnet-20241022", image_req, None).await;

assert!(result.is_err()); // Returns ServiceTypeNotSupported error
```

## Implementation Details

### New Types

1. **ServiceType::Image**: New variant added to the `ServiceType` enum
2. **ImageRequest**: Request structure for image generation
3. **ImageResponse**: Response structure containing generated images

### Adapter Support

- **OpenAI**: Full support for image generation via `/images/generations` endpoint
- **Other adapters**: Return `ServiceTypeNotSupported` error with default trait implementation

### URL Preservation

The OpenAI adapter's `util_get_service_url` function preserves query parameters from the base URL, ensuring compatibility with Azure OpenAI endpoints that require API version parameters.

## Testing

Run the image generation tests:

```bash
cargo test --test tests_p_openai_image
```

Run the example:

```bash
cargo run --example c11-image-generation
```

## Files Modified/Created

### New Files
- `src/chat/image_request.rs`: ImageRequest type definition
- `src/chat/image_response.rs`: ImageResponse type definition
- `examples/c11-image-generation.rs`: Example demonstrating usage
- `tests/tests_p_openai_image.rs`: Test suite for image generation

### Modified Files
- `src/adapter/adapter_types.rs`: Added ServiceType::Image, trait methods
- `src/adapter/adapters/openai/adapter_impl.rs`: Image generation implementation
- `src/adapter/dispatcher.rs`: Dispatcher methods for image generation
- `src/client/client_impl.rs`: Client::exec_image_generation() method
- `src/error.rs`: ServiceTypeNotSupported error variant
- `src/chat/mod.rs`: Export new types
- All adapter implementations: Error handling for unsupported service type

