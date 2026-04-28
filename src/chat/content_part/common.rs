use crate::chat::{Binary, CustomPart, ToolCall, ToolResponse};
use crate::{ModelIden, Result};
use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSummaryText {
	pub text: String,

	#[serde(default = "ReasoningSummaryText::summary_text_type")]
	pub r#type: String,
}

impl ReasoningSummaryText {
	fn summary_text_type() -> String {
		"summary_text".to_string()
	}

	pub fn new(text: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			r#type: Self::summary_text_type(),
		}
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReasoningItem {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,

	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub summary: Vec<ReasoningSummaryText>,

	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub content: Vec<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub encrypted_content: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<String>,
}

impl ReasoningItem {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn from_encrypted_content(encrypted_content: impl Into<String>) -> Self {
		Self {
			encrypted_content: Some(encrypted_content.into()),
			..Default::default()
		}
	}

	pub fn summary_text(&self) -> Option<String> {
		let texts = self
			.summary
			.iter()
			.map(|summary| summary.text.as_str())
			.filter(|text| !text.is_empty())
			.collect::<Vec<_>>();
		if texts.is_empty() { None } else { Some(texts.join("\n")) }
	}

	pub fn size(&self) -> usize {
		self.id.as_ref().map(String::len).unwrap_or_default()
			+ self.summary.iter().map(|summary| summary.text.len()).sum::<usize>()
			+ self.content.iter().map(String::len).sum::<usize>()
			+ self.encrypted_content.as_ref().map(String::len).unwrap_or_default()
			+ self.status.as_ref().map(String::len).unwrap_or_default()
	}
}

/// A single content segment in a chat message.
///
/// Variants cover plain text, binary payloads (e.g., images/PDF), and tool calls/responses.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum ContentPart {
	#[from(String, &String, &str)]
	Text(String),

	#[from]
	Binary(Binary),

	#[from]
	ToolCall(ToolCall),

	#[from]
	ToolResponse(ToolResponse),

	#[from(ignore)]
	ThoughtSignature(String),

	/// Reasoning/thinking content from models that support it (e.g., DeepSeek, kimi).
	/// Adapters extract provider-specific reasoning and normalize it into this variant.
	/// On the request path, adapters hoist these parts back into the provider wire format
	/// (e.g., sibling `reasoning_content` field for OpenAI-compatible providers).
	#[from(ignore)]
	ReasoningContent(String),

	/// Provider-native reasoning item that preserves fields which must remain correlated
	/// for stateless replay, such as OpenAI Responses `summary` and `encrypted_content`.
	#[from]
	ReasoningItem(ReasoningItem),

	#[from]
	Custom(CustomPart),
}

/// Constructors
impl ContentPart {
	/// Create a text content part.
	pub fn from_text(text: impl Into<String>) -> ContentPart {
		ContentPart::Text(text.into())
	}

	/// Create a binary content part from a base64 payload.
	///
	/// - content_type: MIME type (e.g., "image/png", "application/pdf").
	/// - content: base64-encoded bytes.
	/// - name: optional display name or filename.
	pub fn from_binary_base64(
		content_type: impl Into<String>,
		content: impl Into<Arc<str>>,
		name: Option<String>,
	) -> ContentPart {
		ContentPart::Binary(Binary::from_base64(content_type, content, name))
	}

	/// Create a binary content part referencing a URL.
	///
	/// Note: Only some providers accept URL-based inputs.
	pub fn from_binary_url(
		content_type: impl Into<String>,
		url: impl Into<String>,
		name: Option<String>,
	) -> ContentPart {
		ContentPart::Binary(Binary::from_url(content_type, url, name))
	}

	/// Create a binary content part from a file path.
	///
	/// Reads the file, determines the MIME type from the file extension,
	/// and base64-encodes the content.
	///
	/// - file_path: Path to the file to read.
	///
	/// Returns an error if the file cannot be read.
	pub fn from_binary_file(file_path: impl AsRef<Path>) -> Result<ContentPart> {
		Ok(ContentPart::Binary(Binary::from_file(file_path)?))
	}

	pub fn from_custom(data: Value, model_iden: Option<ModelIden>) -> Self {
		ContentPart::Custom(CustomPart { data, model_iden })
	}
}

/// as_.., into_.. Accessors
impl ContentPart {
	/// Borrow the inner text if this part is text.
	pub fn as_text(&self) -> Option<&str> {
		if let ContentPart::Text(content) = self {
			Some(content.as_str())
		} else {
			None
		}
	}

	/// Extract the text, consuming the part.
	pub fn into_text(self) -> Option<String> {
		if let ContentPart::Text(content) = self {
			Some(content)
		} else {
			None
		}
	}

	/// Borrow the tool call if present.
	pub fn as_tool_call(&self) -> Option<&ToolCall> {
		if let ContentPart::ToolCall(tool_call) = self {
			Some(tool_call)
		} else {
			None
		}
	}

	/// Extract the tool call, consuming the part.
	pub fn into_tool_call(self) -> Option<ToolCall> {
		if let ContentPart::ToolCall(tool_call) = self {
			Some(tool_call)
		} else {
			None
		}
	}

	/// Borrow the tool response if present.
	pub fn as_tool_response(&self) -> Option<&ToolResponse> {
		if let ContentPart::ToolResponse(tool_response) = self {
			Some(tool_response)
		} else {
			None
		}
	}

	/// Extract the tool response, consuming the part.
	pub fn into_tool_response(self) -> Option<ToolResponse> {
		if let ContentPart::ToolResponse(tool_response) = self {
			Some(tool_response)
		} else {
			None
		}
	}

	/// Borrow the binary payload if present.
	pub fn as_binary(&self) -> Option<&Binary> {
		if let ContentPart::Binary(binary) = self {
			Some(binary)
		} else {
			None
		}
	}

	// into_binary implemented below
	pub fn into_binary(self) -> Option<Binary> {
		if let ContentPart::Binary(binary) = self {
			Some(binary)
		} else {
			None
		}
	}

	/// Borrow the thought signature if present.
	pub fn as_thought_signature(&self) -> Option<&str> {
		if let ContentPart::ThoughtSignature(thought_signature) = self {
			Some(thought_signature)
		} else {
			None
		}
	}

	/// Extract the thought, consuming the part.
	pub fn into_thought_signature(self) -> Option<String> {
		if let ContentPart::ThoughtSignature(thought_signature) = self {
			Some(thought_signature)
		} else {
			None
		}
	}

	/// Borrow the reasoning content if present.
	pub fn as_reasoning_content(&self) -> Option<&str> {
		if let ContentPart::ReasoningContent(reasoning) = self {
			Some(reasoning)
		} else {
			None
		}
	}

	/// Extract the reasoning content, consuming the part.
	pub fn into_reasoning_content(self) -> Option<String> {
		if let ContentPart::ReasoningContent(reasoning) = self {
			Some(reasoning)
		} else {
			None
		}
	}

	/// Borrow the reasoning item if present.
	pub fn as_reasoning_item(&self) -> Option<&ReasoningItem> {
		if let ContentPart::ReasoningItem(reasoning_item) = self {
			Some(reasoning_item)
		} else {
			None
		}
	}

	/// Extract the reasoning item, consuming the part.
	pub fn into_reasoning_item(self) -> Option<ReasoningItem> {
		if let ContentPart::ReasoningItem(reasoning_item) = self {
			Some(reasoning_item)
		} else {
			None
		}
	}

	/// Borrow the custom part if present.
	pub fn as_custom(&self) -> Option<&CustomPart> {
		if let ContentPart::Custom(custom_part) = self {
			Some(custom_part)
		} else {
			None
		}
	}

	/// Extract the custom part, consuming the part.
	pub fn into_custom(self) -> Option<CustomPart> {
		if let ContentPart::Custom(custom_part) = self {
			Some(custom_part)
		} else {
			None
		}
	}
}

/// Computed accessors
impl ContentPart {
	/// Returns an approximate in-memory size of this `ContentPart`, in bytes.
	///
	/// - For `Text`, `ThoughtSignature`, and `ReasoningContent`: the UTF-8 length of the string.
	/// - For `Binary`: delegates to `Binary::size()`.
	/// - For `ToolCall`: delegates to `ToolCall::size()`.
	/// - For `ToolResponse`: delegates to `ToolResponse::size()`.
	pub fn size(&self) -> usize {
		match self {
			ContentPart::Text(text) => text.len(),
			ContentPart::Binary(binary) => binary.size(),
			ContentPart::ToolCall(tool_call) => tool_call.size(),
			ContentPart::ToolResponse(tool_response) => tool_response.size(),
			ContentPart::ThoughtSignature(thought) => thought.len(),
			ContentPart::ReasoningContent(reasoning) => reasoning.len(),
			ContentPart::ReasoningItem(reasoning_item) => reasoning_item.size(),
			ContentPart::Custom(_value) => 0, // TODO: will need to compute this size
		}
	}
}

/// is_.. Accessors
impl ContentPart {
	#[allow(unused)]
	/// Returns true if this part is text.
	pub fn is_text(&self) -> bool {
		matches!(self, ContentPart::Text(_))
	}

	/// Returns true if this part is binary.
	pub fn is_binary(&self) -> bool {
		matches!(self, ContentPart::Binary(_))
	}

	/// Returns true if this part is a binary image (content_type starts with "image/").
	pub fn is_image(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim().to_ascii_lowercase().starts_with("image/"),
			_ => false,
		}
	}

	/// Returns true if this part is a binary audio (content_type starts with "audio/").
	pub fn is_audio(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim().to_ascii_lowercase().starts_with("audio/"),
			_ => false,
		}
	}

	#[allow(unused)]
	/// Returns true if this part is a PDF binary (content_type equals "application/pdf").
	pub fn is_pdf(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim().eq_ignore_ascii_case("application/pdf"),
			_ => false,
		}
	}

	/// Returns true if this part contains a tool call.
	pub fn is_tool_call(&self) -> bool {
		matches!(self, ContentPart::ToolCall(_))
	}

	/// Returns true if this part contains a tool response.
	pub fn is_tool_response(&self) -> bool {
		matches!(self, ContentPart::ToolResponse(_))
	}

	/// Returns true if this part is a thought.
	pub fn is_thought_signature(&self) -> bool {
		matches!(self, ContentPart::ThoughtSignature(_))
	}

	/// Returns true if this part is reasoning content.
	pub fn is_reasoning_content(&self) -> bool {
		matches!(self, ContentPart::ReasoningContent(_))
	}

	/// Returns true if this part is a provider-native reasoning item.
	pub fn is_reasoning_item(&self) -> bool {
		matches!(self, ContentPart::ReasoningItem(_))
	}

	/// Returns true if this part is custom provider-specific content.
	pub fn is_custom(&self) -> bool {
		matches!(self, ContentPart::Custom(_))
	}
}
