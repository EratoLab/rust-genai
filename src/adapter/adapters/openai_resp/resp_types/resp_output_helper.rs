use crate::chat::{ContentPart, ReasoningItem, ReasoningSummaryText, ToolCall};
use crate::{Error, Result};
use serde_json::Value;
use value_ext::JsonValueExt;

/// Convert a OpenAI response output Item to a ContentPart
///
/// NOTE: At this point this is infallible, will ignore item that cannot be transformed
impl ContentPart {
	pub fn from_resp_output_item(mut item_value: Value) -> Result<Vec<Self>> {
		let mut parts = Vec::new();
		let Some(item_type) = ItemType::from_item_value(&item_value) else {
			return Ok(parts);
		};

		match item_type {
			ItemType::Message => {
				if let Ok(content) = item_value.x_remove::<Vec<Value>>("content") {
					// each content item {}
					for mut content_item in content {
						if let Ok("output_text") = content_item.x_get_str("type")
							&& let Ok(text) = content_item.x_remove::<String>("text")
						{
							parts.push(text.into())
						}
					}
				}
			}
			ItemType::Reasoning => {
				let id = item_value.x_get_str("id").ok().map(ToString::to_string);
				let encrypted_content = item_value.x_get_str("encrypted_content").ok().map(ToString::to_string);
				let status = item_value.x_get_str("status").ok().map(ToString::to_string);
				let summary = item_value
					.get("summary")
					.and_then(Value::as_array)
					.map(|items| {
						items
							.iter()
							.filter_map(|summary_item| {
								if summary_item.x_get_str("type").ok() == Some("summary_text") {
									summary_item
										.x_get_str("text")
										.ok()
										.map(|text| ReasoningSummaryText::new(text.to_string()))
								} else {
									None
								}
							})
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();
				let content = item_value
					.get("content")
					.and_then(Value::as_array)
					.map(|items| {
						items
							.iter()
							.filter_map(|content_item| {
								if content_item.x_get_str("type").ok() == Some("reasoning_text") {
									content_item.x_get_str("text").ok().map(ToString::to_string)
								} else {
									None
								}
							})
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();
				parts.push(ContentPart::ReasoningItem(ReasoningItem {
					id,
					summary,
					content,
					encrypted_content,
					status,
				}));
			}
			ItemType::FunctionCall => {
				let fn_name = item_value.x_remove::<String>("name")?;
				let call_id = item_value.x_remove::<String>("call_id")?;
				let arguments = item_value.x_remove::<String>("arguments")?;
				let fn_arguments: Value =
					serde_json::from_str(&arguments).map_err(|_| Error::InvalidJsonResponseElement {
						info: "tool call arguments is not an object.\nCause",
					})?;

				let tool_call = ToolCall {
					call_id,
					fn_name,
					fn_arguments,
					thought_signatures: None,
				};

				parts.push(tool_call.into());
			}
		}

		Ok(parts)
	}
}

// region:    --- Support Type

/// The managed
enum ItemType {
	Message,
	Reasoning,
	FunctionCall,
}

impl ItemType {
	fn from_item_value(item_value: &Value) -> Option<Self> {
		let typ = item_value.x_get_str("type").ok()?;
		match typ {
			"message" => Some(ItemType::Message),
			"reasoning" => Some(ItemType::Reasoning),
			"function_call" => Some(ItemType::FunctionCall),
			_ => None,
		}
	}
}

// endregion: --- Support Type
