//! Bounded stdin-only operator commands. Never mounted on an HTTP listener.
use super::config;
use baylee_tower::v2::{MAX_STAGES, package::Build};
use baylee_tower_service::Error;
use baylee_tower_store::{Publication, Role, Store};
use central_api::Token;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const MAX_INPUT: usize = baylee_tower_store::MAX_STATE_BYTES + config::MAX_CONFIG + 4096;

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
}

#[derive(Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub(super) enum Reply {
    Operator { id: Uuid },
    Published { cohort: Uuid },
    Done,
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
                    || !(1..=8).contains(archetype)
                    || build.deck.len() > usize::from(baylee_tower::cards::DECK_MAX)
                    || build.active.len() > 6
                {
                    return Err(Error::Contract);
                }
                // The store rechecks the exact persisted cohort. This outer
                // request bound admits both retained and Commander builds.
                let generator = if build.formation.is_some() {
                    baylee_tower::v2::generation::RUN
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
        }
    }
}

#[cfg(test)]
mod tests;
