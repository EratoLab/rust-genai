//! Internal stream event types that serve as intermediaries between the provider event and the GenAI stream event.
//!
//! This allows for flexibility if we want to capture events across providers that do not need to
//! be reflected in the public ChatStream event.
//!
//! NOTE: This might be removed at some point as it may not be needed, and we could go directly to the GenAI stream.

use crate::chat::{ToolCall, Usage};

#[derive(Debug, Default)]
pub struct InterStreamEnd {
	// When `ChatOptions..capture_usage == true`
	pub captured_usage: Option<Usage>,

	// When `ChatOptions..capture_content == true`
	pub captured_content: Option<String>,

	// When `ChatOptions..capture_reasoning_content == true`
	pub captured_reasoning_content: Option<String>,

	// When `ChatOptions..capture_tools == true`
	pub captured_tools: Vec<ToolCall>,
}

/// Intermediary InterReasoningChunk
#[derive(Debug)]
pub enum InterReasoningChunk {
	Content(String),
}

/// Intermediary InterStreamChunkTool
#[derive(Debug)]
pub struct InterStreamChunkTool {
	pub id: String,
	pub name: String,
	pub arguments: String,
}

#[derive(Debug)]
/// Intermediary InterStreamChunk
pub enum InterStreamChunk {
	Content(String),
	Tool(usize, InterStreamChunkTool),
}

/// Intermediary StreamEvent
#[derive(Debug)]
pub enum InterStreamEvent {
	Start,
	Chunk(InterStreamChunk),
	ReasoningChunk(InterReasoningChunk),
	End(InterStreamEnd),
}
