//! A conversation with a model behind an API outside any game: the deck
//! builder's (`crate::deckbuild`). The same key, address, redirect policy,
//! budget and bill as a seat's ([`super::ApiMind`]), with its own
//! instructions and one tool, `act`, whose input is the builder's answer
//! object; a model on an OpenAI-compatible endpoint without tools answers
//! that object as JSON ([`AnswerMode::Json`], [`AnswerMode::JsonSchema`]).

use super::{
    AnswerMode, Api, Credentials, MARGIN, Settings, Tally, ToolResult, anthropic, call,
    is_loopback, json_object, lock, openai, scrub,
};
use crate::cli::Said;
use crate::mind::MindError;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The tool the model answers with.
pub const TOOL: &str = "act";

/// One conversation with a model behind an API.
pub struct DeckChat {
    settings: Settings,
    credentials: Credentials,
    agent: ureq::Agent,
    system: String,
    schema: Value,
    /// The conversation, and what the last reply's tool calls are owed.
    state: tokio::sync::Mutex<(Vec<Value>, Vec<ToolResult>)>,
    tally: Arc<Mutex<Tally>>,
}

impl DeckChat {
    /// A conversation with `settings`' model at `credentials`, its
    /// instructions `system` and its answer held to `schema`.
    #[must_use]
    pub fn new(
        settings: Settings,
        credentials: Credentials,
        system: String,
        schema: Value,
    ) -> Self {
        // As a seat's: no redirect is followed, and nothing leaves this
        // machine but over TLS (`ApiMind::new`).
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .https_only(!is_loopback(&credentials.base))
            .timeout_global(Some(Duration::from_mins(15)))
            .build()
            .new_agent();
        let tally = Tally {
            calls_cap: settings.spend_calls,
            ..Tally::default()
        };
        Self {
            settings,
            credentials,
            agent,
            system,
            schema,
            state: tokio::sync::Mutex::new((Vec::new(), Vec::new())),
            tally: Arc::new(Mutex::new(tally)),
        }
    }

    /// What it has spent, shared.
    #[must_use]
    pub fn tally(&self) -> Arc<Mutex<Tally>> {
        Arc::clone(&self.tally)
    }

    /// The settings it runs with.
    #[must_use]
    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Sends `message`, after `opening` when the conversation begins with
    /// it, within `budget`; the reply's answer object.
    ///
    /// # Errors
    /// [`MindError::Declined`] for a spent budget or a refusal,
    /// [`MindError::Limited`] for a rate limit, [`MindError::Unavailable`]
    /// for any other failure; the key never in the words.
    pub async fn send(
        &self,
        opening: &str,
        message: &str,
        budget: Duration,
    ) -> Result<Said, MindError> {
        let mut state = self.state.lock().await;
        if let Some(why) = lock(&self.tally).spent_under(&self.settings) {
            return Err(MindError::Declined(why));
        }
        let api = self.credentials.api;
        let mut messages = state.0.clone();
        let text = if messages.is_empty() {
            if api == Api::OpenAi {
                messages.push(json!({"role": "system", "content": self.system}));
            }
            format!("{opening}\n\n{message}")
        } else {
            message.to_string()
        };
        match api {
            Api::Anthropic => anthropic::user(&mut messages, &state.1, None, &text),
            Api::OpenAi => openai::user(&mut messages, &state.1, None, &text),
        }
        let (url, body) = self.request(&messages);
        let bytes = body.to_string().into_bytes();
        let worst = self.settings.worst(bytes.len());
        {
            let mut tally = lock(&self.tally);
            if let Err(why) = tally.hold(worst, &self.settings) {
                tally.spent = true;
                return Err(MindError::Declined(why));
            }
        }
        let headers = self.headers();
        let agent = self.agent.clone();
        let mode = self.settings.answer;
        let timeout = budget.saturating_sub(MARGIN).max(Duration::from_secs(1));
        let answered = tokio::task::spawn_blocking(move || {
            call(&agent, &url, &headers, &bytes, timeout, api, mode)
        })
        .await
        .map_err(|e| MindError::Unavailable(format!("the call's task ended: {e}")))?;
        let key = self.credentials.key.as_ref();
        let reply = match answered {
            Ok(reply) => {
                lock(&self.tally).back(worst, Ok(reply.usage), self.settings.price);
                reply
            }
            Err(failed) => {
                lock(&self.tally).back(worst, Err(failed.unknown_bill), self.settings.price);
                return Err(failed.error(key));
            }
        };
        if reply.stop == super::Stop::Refusal {
            return Err(MindError::Declined("the model refused to answer".into()));
        }
        messages.push(reply.assistant.clone());
        // Every call of the reply is answered in the next turn, as the API
        // asks; the object is read from the first `act`.
        let owed = reply
            .calls
            .iter()
            .filter(|c| !c.id.is_empty())
            .map(|c| ToolResult {
                id: c.id.clone(),
                content: "received".into(),
                is_error: false,
            })
            .collect();
        let value = reply
            .calls
            .iter()
            .find(|c| c.name == TOOL || (mode.is_json() && c.name == "decide"))
            .map(|c| c.input.clone())
            .filter(Value::is_object)
            .or_else(|| json_object(&reply.text));
        *state = (messages, owed);
        Ok(Said {
            value,
            text: scrub(&reply.text, key),
            usage: Some(reply.usage),
        })
    }

    /// Where the request goes, and its body.
    fn request(&self, messages: &[Value]) -> (String, Value) {
        let base = &self.credentials.base;
        let tool_description = "Your one answer each turn: an object with one action.";
        match self.credentials.api {
            Api::Anthropic => {
                let mut body = json!({
                    "model": self.settings.model,
                    "max_tokens": self.settings.max_tokens,
                    "system": [{"type": "text", "text": self.system, "cache_control": {"type": "ephemeral"}}],
                    "messages": messages,
                    "tools": [{"name": TOOL, "description": tool_description, "input_schema": self.schema}],
                    "tool_choice": {"type": "auto"},
                    "thinking": {"type": "adaptive", "display": "summarized"},
                    "cache_control": {"type": "ephemeral"},
                });
                if let Some(effort) = &self.settings.effort {
                    body["output_config"] = json!({"effort": effort});
                }
                (anthropic::url(base), body)
            }
            Api::OpenAi => {
                let mut body = json!({"model": self.settings.model, "messages": messages});
                body[openai::reply_limit(base)] = json!(self.settings.max_tokens);
                match self.settings.answer {
                    AnswerMode::Tools => {
                        body["tools"] = json!([{"type": "function", "function": {
                            "name": TOOL, "description": tool_description, "parameters": self.schema,
                        }}]);
                        body["tool_choice"] = json!("auto");
                    }
                    AnswerMode::Json => body["response_format"] = json!({"type": "json_object"}),
                    AnswerMode::JsonSchema => {
                        body["response_format"] = json!({
                            "type": "json_schema",
                            "json_schema": {"name": TOOL, "schema": self.schema},
                        });
                    }
                }
                if let Some(effort) = &self.settings.effort {
                    body["reasoning_effort"] = json!(effort);
                }
                (openai::url(base), body)
            }
        }
    }

    /// The request's headers, the key among them.
    fn headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![("content-type", "application/json".to_string())];
        match self.credentials.api {
            Api::Anthropic => {
                headers.push(("anthropic-version", anthropic::VERSION.to_string()));
                if let Some(key) = &self.credentials.key {
                    headers.push(("x-api-key", key.expose().to_string()));
                }
            }
            Api::OpenAi => {
                if let Some(key) = &self.credentials.key {
                    headers.push(("authorization", format!("Bearer {}", key.expose())));
                }
            }
        }
        headers
    }
}
