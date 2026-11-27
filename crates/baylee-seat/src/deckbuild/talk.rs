//! Whom the deck builder talks to: a model behind an API
//! ([`crate::llm::deckchat::DeckChat`]) or an agent CLI
//! ([`crate::cli::CliMind::converse`]), each as a [`Chat`], and a profile's
//! fallbacks after it ([`Chain`], `docs/llm-seat.md` §"A fallback
//! profile"): when one is rate limited, out of its quota or failing, the
//! next takes the same message over, told the work so far.

use crate::cli::{CliMind, Said};
use crate::llm::Tally;
use crate::llm::deckchat::DeckChat;
use crate::mind::MindError;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A reply on its way.
pub type Replying<'a> = Pin<Box<dyn Future<Output = Result<Said, MindError>> + Send + 'a>>;

/// One model's conversation.
pub trait Chat: Send + Sync {
    /// Sends `message`, after `opening` when the conversation begins with
    /// it (its first message, or one begun again after a loss), within
    /// `budget`.
    fn send<'a>(&'a self, opening: &'a str, message: &'a str, budget: Duration) -> Replying<'a>;

    /// What it is, for the record: the model's id, and its profile and
    /// effort where it has them.
    fn who(&self) -> Who;

    /// What it has spent.
    fn tally(&self) -> Arc<Mutex<Tally>>;
}

/// Which model a chat talks to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Who {
    /// The model's id (`claude-opus-5-5`, `claude:opus`).
    pub model: String,
    /// The profile it runs under.
    pub profile: Option<String>,
    /// Its effort.
    pub effort: Option<String>,
}

impl Chat for DeckChat {
    fn send<'a>(&'a self, opening: &'a str, message: &'a str, budget: Duration) -> Replying<'a> {
        Box::pin(DeckChat::send(self, opening, message, budget))
    }

    fn who(&self) -> Who {
        let settings = self.settings();
        Who {
            model: settings.model.clone(),
            profile: settings.profile.clone(),
            effort: settings.effort.clone(),
        }
    }

    fn tally(&self) -> Arc<Mutex<Tally>> {
        DeckChat::tally(self)
    }
}

/// An agent CLI's conversation, held by its own process.
pub struct CliChat {
    mind: CliMind,
    conversation: String,
}

impl CliChat {
    /// A conversation with `mind`, which was made with the builder's brief
    /// ([`crate::cli::CliMind::with_brief`]).
    #[must_use]
    pub fn new(mind: CliMind) -> Self {
        Self {
            mind,
            conversation: format!("deckbuild-{}", uuid::Uuid::now_v7()),
        }
    }
}

impl Chat for CliChat {
    fn send<'a>(&'a self, opening: &'a str, message: &'a str, budget: Duration) -> Replying<'a> {
        Box::pin(
            self.mind
                .converse(&self.conversation, opening, message, budget),
        )
    }

    fn who(&self) -> Who {
        let settings = self.mind.settings();
        Who {
            model: settings.model.clone(),
            profile: settings.profile.clone(),
            effort: settings.effort.clone(),
        }
    }

    fn tally(&self) -> Arc<Mutex<Tally>> {
        self.mind.tally()
    }
}

/// A model and the fallbacks after it, first to last. A model that is
/// limited or unavailable hands over for good: the next one is asked the
/// same message, as the first of its own conversation. A model that
/// declines (its budget spent, a refusal) hands over too.
pub struct Chain {
    chats: Vec<Box<dyn Chat>>,
    at: Mutex<usize>,
    /// What each hand-over said, for the terminal.
    pub said: Mutex<Vec<String>>,
}

impl Chain {
    /// `chats`, the first playing.
    #[must_use]
    pub fn new(chats: Vec<Box<dyn Chat>>) -> Self {
        Self {
            chats,
            at: Mutex::new(0),
            said: Mutex::new(Vec::new()),
        }
    }

    /// The chat answering now.
    #[must_use]
    pub fn current(&self) -> Option<&dyn Chat> {
        let at = *crate::llm::lock(&self.at);
        self.chats.get(at).map(AsRef::as_ref)
    }

    /// Every chat's spend, first to last.
    #[must_use]
    pub fn tallies(&self) -> Vec<Arc<Mutex<Tally>>> {
        self.chats.iter().map(|chat| chat.tally()).collect()
    }

    /// `message` to the chat answering now; on its failure, to the next.
    /// `opening` is what a conversation that begins hears first.
    ///
    /// # Errors
    /// The last failure, when no chat is left.
    pub async fn send(
        &self,
        opening: &str,
        message: &str,
        budget: Duration,
    ) -> Result<Said, String> {
        loop {
            let at = *crate::llm::lock(&self.at);
            let Some(chat) = self.chats.get(at) else {
                return Err("no model is left to answer".into());
            };
            match chat.send(opening, message, budget).await {
                Ok(said) => return Ok(said),
                Err(error) => {
                    let who = chat.who().model;
                    let why = error.to_string();
                    let next = self.chats.get(at + 1).map(|c| c.who().model);
                    let line = match &next {
                        Some(next) => format!("{who} cannot answer ({why}); {next} takes over"),
                        None => format!("{who} cannot answer ({why}), and no fallback is left"),
                    };
                    crate::llm::lock(&self.said).push(line);
                    if next.is_none() {
                        return Err(format!("{who}: {why}"));
                    }
                    *crate::llm::lock(&self.at) = at + 1;
                }
            }
        }
    }
}
