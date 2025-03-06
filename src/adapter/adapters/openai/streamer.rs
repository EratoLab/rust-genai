use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{
	InterReasoningChunk, InterStreamChunk, InterStreamChunkTool, InterStreamEnd, InterStreamEvent,
};
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::AdapterKind;
use crate::chat::{ChatOptionsSet, ToolCall};
use crate::{Error, ModelIden, Result};
use reqwest_eventsource::{Event, EventSource};
use serde::Deserialize;
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

pub struct OpenAIStreamer {
	inner: EventSource,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
	partial_openai_tool_call: Option<OpenAIToolCall>,
}

impl OpenAIStreamer {
	// TODO: Problem - need the ChatOptions `.capture_content` and `.capture_usage`
	pub fn new(inner: EventSource, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
			partial_openai_tool_call: None,
		}
	}
}

impl futures::Stream for OpenAIStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			// The last poll was definitely the end, so end the stream.
			// This will prevent triggering a stream ended error
			return Poll::Ready(None);
		}
		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => {
					tracing::trace!("Message: {:?}", message);

					// -- End Message
					// According to OpenAI Spec, this is the end message
					if message.data == "[DONE]" {
						self.done = true;

						// -- Build the usage and captured_content
						// TODO: Needs to clarify wh for usage we do not adopt the same strategy from captured content below
						let captured_usage = if self.options.capture_usage {
							self.captured_data.usage.take()
						} else {
							None
						};

						// if there is still a tool call that was in progress, now is completed, so return it.
						if self.options.capture_tools {
							if let Some(tool) = self.partial_openai_tool_call.take() {
								let tool: ToolCall = tool.into();
								self.captured_data.tools.push(tool.clone());
							}
						}

						let inter_stream_end = InterStreamEnd {
							captured_usage,
							captured_content: self.captured_data.content.take(),
							captured_reasoning_content: self.captured_data.reasoning_content.take(),
							captured_tools: self.captured_data.tools.clone(),
						};

						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}

					// -- Other Content Messages
					// Parse to get the choice
					let mut message_data: Value =
						serde_json::from_str(&message.data).map_err(|serde_error| Error::StreamParse {
							model_iden: self.options.model_iden.clone(),
							serde_error,
						})?;

					let adapter_kind = self.options.model_iden.adapter_kind;
					// If we have a first choice, then it's a normal message
					if let Ok(Some(first_choice)) = message_data.x_take::<Option<Value>>("/choices/0") {
						// -- Finish Reason
						// If finish_reason exists, it's the end of this choice.
						// Since we support only a single choice, we can proceed,
						// as there might be other messages, and the last one contains data: `[DONE]`
						// NOTE: xAI has no `finish_reason` when not finished, so, need to just account for both null/absent
						if let Ok(_finish_reason) = first_choice.clone().x_take::<String>("finish_reason") {
							// NOTE: For Groq, the usage is captured when finish_reason indicates stopping, and in the `/x_groq/usage`
							if self.options.capture_usage {
								match adapter_kind {
									AdapterKind::Groq => {
										let usage = message_data
											.x_take("/x_groq/usage")
											.map(OpenAIAdapter::into_usage)
											.unwrap_or_default(); // permissive for now
										self.captured_data.usage = Some(usage)
									}
									AdapterKind::Xai | AdapterKind::DeepSeek => {
										let usage = message_data
											.x_take("usage")
											.map(OpenAIAdapter::into_usage)
											.unwrap_or_default();
										self.captured_data.usage = Some(usage)
									}
									_ => (), // do nothing, will be captured the OpenAI way
								}
							}

							continue;
						}
						// -- Content
						// If there is no finish_reason but there is some content, we can get the delta content and send the Internal Stream Event
						if let Ok(Some(content)) = first_choice.clone().x_take::<Option<String>>("/delta/content") {
							// Add to the captured_content if chat options allow it
							if self.options.capture_content {
								match self.captured_data.content {
									Some(ref mut c) => c.push_str(&content),
									None => self.captured_data.content = Some(content.clone()),
								}
							}

							// Return the Event
							return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(InterStreamChunk::Content(content)))));
						}

						// -- Tool Call
						// there will be always only one tool_call during streaming
						if let Ok(Some(tool)) =
							first_choice.clone().x_take::<Option<OpenAIToolCall>>("/delta/tool_calls/0")
						{
							// Example of a tool call event:
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"id":"call_VkT1Z57SU75JNIOCxzGZnYVd","type":"function","function":{"name":"get_weather","arguments":""}}],"refusal":null},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\""}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"city"}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\":\""}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"Tokyo"}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\",\""}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"country"}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\":\""}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"Japan"}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\",\""}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"unit"}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\":\""}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"C"}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"}"}}]},"logprobs":null,"finish_reason":null}]}"
							// "{"id":"chatcmpl-B7jpM7pmGIMXiYc8vnkfOTZQzC19e","object":"chat.completion.chunk","created":1741184156,"model":"gpt-4o-mini-2024-07-18","service_tier":"default","system_fingerprint":"fp_06737a9306","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"tool_calls"}]}"
							// [DONE]

							if let Some(mut p) = self.partial_openai_tool_call.take() {
								if tool.index == p.index {
									p.id.push_str(tool.id.as_str());
									p.function.name.push_str(tool.function.name.as_str());
									p.function.arguments.push_str(tool.function.arguments.as_str());
									self.partial_openai_tool_call.replace(p);
								} else {
									self.partial_openai_tool_call.replace(tool.clone());

									if self.options.capture_tools {
										self.captured_data.tools.push(p.clone().into());
									}
								}
							} else {
								self.partial_openai_tool_call.replace(tool.clone());
							}

							// proceed with the next event
							return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(tool.into()))));
						}
						// -- Reasoning Content

						if let Ok(Some(reasoning_content)) =
							first_choice.clone().x_take::<Option<String>>("/delta/reasoning_content")
						{
							// Add to the captured_content if chat options allow it
							if self.options.capture_reasoning_content {
								match self.captured_data.reasoning_content {
									Some(ref mut c) => c.push_str(&reasoning_content),
									None => self.captured_data.reasoning_content = Some(reasoning_content.clone()),
								}
							}

							// Return the Event
							return Poll::Ready(Some(Ok(InterStreamEvent::ReasoningChunk(
								InterReasoningChunk::Content(reasoning_content),
							))));
						}

						// If we do not have content, then log a trace message
						tracing::warn!("EMPTY CHOICE CONTENT");
					}
					// -- Usage message
					else {
						// If it's not Groq, xAI, DeepSeek the usage is captured at the end when choices are empty or null
						if !matches!(adapter_kind, AdapterKind::Groq)
							&& !matches!(adapter_kind, AdapterKind::Xai)
							&& !matches!(adapter_kind, AdapterKind::DeepSeek)
							&& self.captured_data.usage.is_none() // this might be redundant
							&& self.options.capture_usage
						{
							// permissive for now
							let usage = message_data.x_take("usage").map(OpenAIAdapter::into_usage).unwrap_or_default();
							self.captured_data.usage = Some(usage);
						}
					}
				}
				Some(Err(err)) => {
					tracing::error!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::ReqwestEventSource(err))));
				}
				None => {
					return Poll::Ready(None);
				}
			}
		}
		Poll::Pending
	}
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAIToolCallFunction {
	#[serde(default)]
	name: String,
	#[serde(default)]
	arguments: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAIToolCall {
	index: usize,
	#[serde(default)]
	id: String,
	#[serde(default)]
	function: OpenAIToolCallFunction,
}

impl From<OpenAIToolCall> for InterStreamChunk {
	fn from(tool: OpenAIToolCall) -> Self {
		InterStreamChunk::Tool(tool.index, tool.into())
	}
}

impl From<OpenAIToolCall> for InterStreamChunkTool {
	fn from(tool: OpenAIToolCall) -> Self {
		InterStreamChunkTool {
			id: tool.id,
			name: tool.function.name,
			arguments: tool.function.arguments,
		}
	}
}

impl From<OpenAIToolCall> for ToolCall {
	fn from(tool: OpenAIToolCall) -> Self {
		ToolCall {
			call_id: tool.id.clone(),
			fn_name: tool.function.name.clone(),
			fn_arguments: serde_json::from_str(&tool.function.arguments).unwrap_or_default(),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn test_deserialize_tool_call() {
		let tool_call = r#"{"index":0,"id":"call_VkT1Z57SU75JNIOCxzGZnYVd","type":"function","function":{"name":"get_weather","arguments":""}}"#;
		let tool_call: OpenAIToolCall = serde_json::from_str(tool_call).unwrap();
		assert_eq!(tool_call.index, 0);
		assert_eq!(tool_call.id, "call_VkT1Z57SU75JNIOCxzGZnYVd");
		assert_eq!(tool_call.function.name, "get_weather");
		assert_eq!(tool_call.function.arguments, "");
	}

	#[test]
	fn test_deserialize_tool_call_function() {
		let tool_call_function = r#"{"name":"get_weather","arguments":""}"#;
		let tool_call_function: OpenAIToolCallFunction = serde_json::from_str(tool_call_function).unwrap();
		assert_eq!(tool_call_function.name, "get_weather");
		assert_eq!(tool_call_function.arguments, "");
	}

	#[test]
	fn test_deserialize_tool_call_function_with_arguments() {
		let tool_call_function =
			r#"{"name":"get_weather","arguments":"{\"city\":\"Tokyo\",\"country\":\"Japan\",\"unit\":\"C\"}"}"#;
		let tool_call_function: OpenAIToolCallFunction = serde_json::from_str(tool_call_function).unwrap();
		assert_eq!(tool_call_function.name, "get_weather");
		assert_eq!(
			tool_call_function.arguments,
			"{\"city\":\"Tokyo\",\"country\":\"Japan\",\"unit\":\"C\"}"
		);
	}

	#[test]
	fn test_partial_deserialize_tool() {
		let tool_call = r#"{"index":0,"function":{"arguments":"{\""}}"#;
		let tool_call: OpenAIToolCall = serde_json::from_str(tool_call).unwrap();
		assert_eq!(tool_call.index, 0);
		assert_eq!(tool_call.id, "");
		assert_eq!(tool_call.function.name, "");
		assert_eq!(tool_call.function.arguments, "{\"");
	}
}
