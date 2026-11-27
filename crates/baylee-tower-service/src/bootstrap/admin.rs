//! Bounded stdin-only operator commands. Never mounted on an HTTP listener.
use super::config;
use baylee_tower::v2::{MAX_STAGES, package::Build};
use baylee_tower_service::Error;
use baylee_tower_store::{Erasure, Publication, Recovery, Role, Store};
use central_api::Token;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) const MAX_INPUT: usize =
    baylee_tower_store::MAX_PUBLICATION_BYTES + config::MAX_CONFIG + 4096;

/// Accounts one `erase_accounts` takes; a longer list goes in several calls.
pub(super) const MAX_ERASED: usize = 1024;

/// Every `--admin` action this build knows, as `--modes` names them, so that a
/// tool can ask before it relies on one.
pub(super) const ACTIONS: [&str; 9] = [
    "provision",
    "revoke",
    "approve_host",
    "publish",
    "publish_house",
    "pause_enrollment",
    "supervise",
    "grant_runtime",
    "erase_accounts",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    pub configuration: config::Config,
    pub operation: Operation,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Operation {
    Provision {
        name: String,
        role: Role,
        #[serde(deserialize_with = "config::token")]
        credential: Token,
    },
    Revoke {
        operator: Uuid,
    },
    ApproveHost {
        host: Uuid,
        origin: String,
    },
    Publish {
        publication: Box<Publication>,
    },
    PublishHouse {
        #[serde(deserialize_with = "config::token")]
        publisher: Token,
        cohort: Uuid,
        stage: u8,
        archetype: u8,
        build: Build,
    },
    PauseEnrollment {
        cohort: Uuid,
    },
    /// One closed technical transition under a live Supervisor credential.
    Supervise {
        #[serde(deserialize_with = "config::token")]
        supervisor: Token,
        run: Uuid,
        revision: u64,
        recovery: Supervision,
    },
    /// Renew the runtime role's reviewed grants after a migration.
    GrantRuntime {
        role: String,
    },
    /// Erase accounts' Tower data, each in its own transaction: on individual
    /// requests, and again after a restore from an older backup.
    EraseAccounts {
        accounts: Vec<Uuid>,
    },
}

/// The store's closed recovery, spelt so an extra field is refused: serde would
/// ignore one beside a unit variant of a tagged enum.
#[derive(Clone, Copy, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Supervision {
    Quarantine {},
    Resume { host: Option<Uuid> },
    Interrupt {},
}

impl From<Supervision> for Recovery {
    fn from(value: Supervision) -> Self {
        match value {
            Supervision::Quarantine {} => Self::Quarantine,
            Supervision::Resume { host } => Self::Resume { host },
            Supervision::Interrupt {} => Self::Interrupt,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub(super) enum Reply {
    Operator {
        id: Uuid,
    },
    Published {
        cohort: Uuid,
    },
    Supervised {
        revision: u64,
    },
    /// Counts only: the reply names no account.
    Erased {
        removed: u64,
        absent: u64,
        busy: u64,
    },
    Done,
}

impl Reply {
    /// Whether the action did all it was asked: an erasure that met a duel
    /// under way did not, and the same list runs again later.
    pub(super) fn complete(&self) -> bool {
        !matches!(self, Self::Erased { busy, .. } if *busy > 0)
    }
}

impl Input {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_INPUT {
            return Err(Error::Contract);
        }
        let input: Self = serde_json::from_slice(bytes).map_err(|_| Error::Contract)?;
        input.configuration.validate()?;
        if input.configuration.service.is_some() {
            return Err(Error::Contract);
        }
        input.operation.validate(input.configuration.development)?;
        Ok(input)
    }
}

fn uuid7(id: Uuid) -> Result<(), Error> {
    if id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122 {
        Ok(())
    } else {
        Err(Error::Contract)
    }
}

impl Operation {
    fn validate(&self, development: bool) -> Result<(), Error> {
        match self {
            Self::Provision { name, .. } => {
                if name.is_empty()
                    || name.len() > 160
                    || name.trim() != name
                    || name.chars().any(char::is_control)
                {
                    return Err(Error::Contract);
                }
            }
            Self::Revoke { operator } => uuid7(*operator)?,
            Self::ApproveHost { host, origin } => {
                uuid7(*host)?;
                central_api::gateway::validate_origin(origin, development)
                    .map_err(|_| Error::Contract)?;
            }
            Self::Publish { publication } => publication.validate()?,
            Self::PublishHouse {
                cohort,
                stage,
                archetype,
                build,
                ..
            } => {
                uuid7(*cohort)?;
                if !(1..=MAX_STAGES).contains(stage)
                    || !(1..=64).contains(archetype)
                    || build.deck.len() > usize::from(baylee_tower::cards::DECK_MAX)
                    || build.active.len() > 6
                {
                    return Err(Error::Contract);
                }
                // The store rechecks the exact persisted cohort. This outer
                // request bound admits both retained and Commander builds: a
                // Commander build is bounded by the widest run generation, as
                // `Build::validate` does, or a runic relic is refused here.
                let generator = if build.formation.is_some() {
                    baylee_tower::v2::generation::RUNIC
                } else {
                    3
                };
                baylee_tower::v2::loadout::resources_for(
                    &build.active,
                    build.potions,
                    0,
                    generator,
                )
                .map_err(|_| Error::Contract)?;
            }
            Self::PauseEnrollment { cohort } => uuid7(*cohort)?,
            Self::EraseAccounts { accounts } => {
                if accounts.is_empty() || accounts.len() > MAX_ERASED {
                    return Err(Error::Contract);
                }
                for account in accounts {
                    uuid7(*account)?;
                }
            }
            Self::GrantRuntime { role } => {
                if !baylee_tower_store::valid_schema(role) {
                    return Err(Error::Contract);
                }
            }
            Self::Supervise { run, recovery, .. } => {
                uuid7(*run)?;
                if let Supervision::Resume { host: Some(host) } = recovery {
                    uuid7(*host)?;
                }
            }
        }
        Ok(())
    }

    pub async fn execute(self, store: &Store, development: bool) -> Result<Reply, Error> {
        // SQL ownership is independent of an application operator credential. A
        // host/verifier/publisher secret cannot turn runtime grants into admin.
        store.administration_ready().await?;
        self.validate(development)?;
        match self {
            Self::Provision {
                name,
                role,
                credential,
            } => Ok(Reply::Operator {
                id: store
                    .provision_operator(&name, role, credential.digest())
                    .await?,
            }),
            Self::Revoke { operator } => {
                store.revoke_operator(operator).await?;
                Ok(Reply::Done)
            }
            Self::ApproveHost { host, origin } => {
                store.approve_host(host, &origin, development).await?;
                Ok(Reply::Done)
            }
            Self::Publish { publication } => Ok(Reply::Published {
                cohort: store.publish(&publication).await?,
            }),
            Self::PublishHouse {
                publisher,
                cohort,
                stage,
                archetype,
                build,
            } => {
                let principal = store.authenticate_operator(publisher.digest()).await?;
                store
                    .publish_house(&principal, cohort, stage, archetype, &build)
                    .await?;
                Ok(Reply::Done)
            }
            Self::PauseEnrollment { cohort } => {
                store.pause_enrollment(cohort).await?;
                Ok(Reply::Done)
            }
            Self::Supervise {
                supervisor,
                run,
                revision,
                recovery,
            } => {
                let principal = store.authenticate_operator(supervisor.digest()).await?;
                let snapshot = store
                    .supervise(&principal, run, revision, recovery.into())
                    .await?;
                Ok(Reply::Supervised {
                    revision: snapshot.state.revision,
                })
            }
            Self::GrantRuntime { role } => {
                store.grant_runtime(&role).await?;
                Ok(Reply::Done)
            }
            Self::EraseAccounts { accounts } => {
                let (mut removed, mut absent, mut busy) = (0, 0, 0);
                // Stopped by a failure, what was erased stays erased; the same
                // list runs again.
                for account in accounts.into_iter().collect::<BTreeSet<_>>() {
                    match store.erase_account(account).await? {
                        Erasure::Removed => removed += 1,
                        Erasure::Absent => absent += 1,
                        Erasure::Busy => busy += 1,
                    }
                }
                Ok(Reply::Erased {
                    removed,
                    absent,
                    busy,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
