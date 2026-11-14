//! Private gateway/engine transport. No credentials or executable paths cross it.
use crate::host::{Acknowledged, Prefix};
use baylee_protocol::v1::{Envelope, TowerFrame, envelope::Msg};
use baylee_tower::{
    Refusal,
    cards::Catalog,
    v2::{
        execution::{Execution, MAX_BATCH_CHUNKS, MAX_CHUNK_BYTES},
        package::Package,
        verification::Resources,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod assembly;
pub use assembly::Assembly;

/// Private setup ceiling; keeps every engine frame below the existing socket cap.
pub const MAX_CONTROL_BYTES: usize = 256 * 1024;

/// Authority-derived immutable setup, without host/operator credentials.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    /// Gateway game identity, unrelated to authorization.
    pub game: Uuid,
    /// Frozen authority package.
    pub package: Package,
    /// Reviewed immutable catalog.
    pub catalog: Catalog,
    /// Current authority assignment generation.
    pub fence: u64,
    /// Exact prefix to recover before admitting new input.
    pub prefix: Prefix,
    /// Original reservation's private record budget.
    pub byte_limit: u64,
    /// Presentation only, in human/opponent order.
    pub names: [String; 2],
    /// Bounded technical reconstruction allocation, not a rules time limit.
    pub resources: Resources,
}

impl Setup {
    /// Validate before starting rules work or allocating a recovery buffer.
    /// # Errors
    /// Malformed identity, package, commitment, names or resource allocation.
    pub fn validate(&self) -> Result<(), Refusal> {
        self.resources.validate()?;
        if self.game.get_version_num() != 7
            || self.game.get_variant() != uuid::Variant::RFC4122
            || self.fence == 0
            || self.resources.bytes < self.byte_limit
            || self.names.iter().any(|name| {
                name.is_empty() || name.len() > 160 || name.chars().any(char::is_control)
            })
        {
            return Err(Refusal::Invalid);
        }
        let mut state = Execution::new(self.package.digest(&self.catalog)?, self.byte_limit)?;
        state.fence = self.fence;
        state.prefix = self.prefix;
        state.validate()
    }
}

/// Strict private controls. Endpoint state machines also enforce direction/order.
/// Neither side can authorize a win, currency or an arbitrary engine mutation.
#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    /// Gateway → engine, exactly once after authenticated engine hello.
    Start {
        /// Frozen setup.
        setup: Box<Setup>,
    },
    /// Engine → gateway, pull one retained chunk while reconstructing.
    Read {
        /// Exact zero-based retained stream index.
        index: u64,
    },
    /// Gateway → engine, one raw retained chunk in `TowerFrame.data`.
    Recovery {
        /// Exact requested index.
        index: u64,
    },
    /// Engine → gateway, one part of a single atomic operation, in `data`.
    Append {
        /// Current assignment generation.
        fence: u64,
        /// Previous durable head shared by every part.
        expected: Prefix,
        /// Total parts; the gateway commits only after the last.
        chunks: u8,
        /// Zero-based part index.
        index: u8,
    },
    /// Gateway → engine, complete durable operation, never an individual part.
    Acknowledged {
        /// Exact committed fence and head.
        acknowledgement: Acknowledged,
    },
    /// Engine → gateway, terminal candidate; only independent replay awards it.
    Seal {
        /// Complete acknowledged prefix.
        expected: Prefix,
    },
    /// Gateway → engine, sealing committed; no rule outcome or reward implied.
    Sealed {
        /// Same complete prefix and generation.
        acknowledgement: Acknowledged,
    },
    /// Engine → gateway, only after the sealed ACK and final visible frames.
    /// Also closes an already-terminal recovered game without inventing a new end.
    Finished {},
}

impl Control {
    /// Encode a single bounded engine frame. Raw data never becomes JSON numbers.
    /// # Errors
    /// Malformed payload, setup or oversize control/data.
    pub fn envelope(&self, data: Vec<u8>) -> Result<Envelope, Refusal> {
        self.validate(&data)?;
        let control = serde_json::to_vec(self).map_err(|_| Refusal::Invalid)?;
        if control.len() > MAX_CONTROL_BYTES {
            return Err(Refusal::Capacity);
        }
        Ok(Envelope {
            msg: Some(Msg::TowerFrame(TowerFrame { control, data })),
        })
    }

    /// Decode only bounded strict controls; unknown fields and extra data fail.
    /// # Errors
    /// Malformed framing, setup or payload shape.
    pub fn decode(frame: TowerFrame) -> Result<(Self, Vec<u8>), Refusal> {
        if frame.control.len() > MAX_CONTROL_BYTES || frame.data.len() > MAX_CHUNK_BYTES {
            return Err(Refusal::Capacity);
        }
        let control: Self = serde_json::from_slice(&frame.control).map_err(|_| Refusal::Invalid)?;
        control.validate(&frame.data)?;
        Ok((control, frame.data))
    }

    fn validate(&self, data: &[u8]) -> Result<(), Refusal> {
        let chunk = matches!(self, Self::Recovery { .. } | Self::Append { .. });
        if (chunk && (data.is_empty() || data.len() > MAX_CHUNK_BYTES))
            || (!chunk && !data.is_empty())
        {
            return Err(Refusal::Capacity);
        }
        match self {
            Self::Start { setup } => setup.validate(),
            Self::Append {
                fence,
                chunks,
                index,
                ..
            } if *fence == 0 || usize::from(*chunks) > MAX_BATCH_CHUNKS || *index >= *chunks => {
                Err(Refusal::Invalid)
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
