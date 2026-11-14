//! Dedicated Tower operator credentials; gateway registration alone grants no role.
use super::{ConnectionTrait, Error, Refusal, Store, Uuid, column, decode, encode};
use baylee_protocol::execution::{Identity, valid_capabilities};
use serde::{Deserialize, Serialize};

/// Closed, separate authority roles; no role implies another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Approved gateway/execution operator; may execute frozen reservations.
    Host,
    /// Independent record adjudication; cannot shop or change packages.
    Verifier,
    /// Reviewed immutable content publication; cannot fabricate results.
    Publisher,
    /// Explicit technical recovery; cannot grant rules wins or currency.
    Supervisor,
}

impl Role {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Verifier => "verifier",
            Self::Publisher => "publisher",
            Self::Supervisor => "supervisor",
        }
    }

    fn parse(name: &str) -> Result<Self, Error> {
        [
            Self::Host,
            Self::Verifier,
            Self::Publisher,
            Self::Supervisor,
        ]
        .into_iter()
        .find(|role| role.name() == name)
        .ok_or(Error::Unavailable)
    }
}

/// A verified credential lookup. Private fields prevent caller-selected roles;
/// each operation still rechecks enabled state/role within its own transaction.
#[derive(Clone)]
pub struct Principal {
    pub(super) id: Uuid,
    pub(super) role: Role,
}

impl Principal {
    /// Public operator identity, never a credential.
    pub const fn id(&self) -> Uuid {
        self.id
    }
    /// Exact authenticated role; no implicit inheritance.
    pub const fn role(&self) -> Role {
        self.role
    }
}

impl Store {
    /// Offline operator provisioning with migration/admin access, never a player route.
    /// The caller uses the reviewed token library's independent secret and digest;
    /// neither a Central session nor a gateway credential is reused.
    /// # Errors
    /// Invalid name, duplicate credential digest or storage failure.
    pub async fn provision_operator(
        &self,
        name: &str,
        role: Role,
        credential_digest: [u8; 32],
    ) -> Result<Uuid, Error> {
        if name.is_empty()
            || name.len() > 160
            || name.trim() != name
            || name.chars().any(char::is_control)
        {
            return Err(Refusal::Invalid.into());
        }
        let row = self.db.query_one_raw(self.statement("INSERT INTO @tower.operators(name,role,credential_digest) VALUES($1,$2,$3) ON CONFLICT(credential_digest) DO NOTHING RETURNING id",
            vec![name.into(), role.name().into(), credential_digest.to_vec().into()])).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Conflict)?;
        column(&row, "id")
    }

    /// Resolve a dedicated Tower operator secret's digest; never accept a claimed role.
    /// The transport must bound/rate-limit credential parsing before calling this.
    /// # Errors
    /// Unknown/disabled credential or storage failure.
    pub async fn authenticate_operator(&self, digest: [u8; 32]) -> Result<Principal, Error> {
        let row = self
            .db
            .query_one_raw(self.statement(
                "SELECT id,role FROM @tower.operators WHERE credential_digest=$1 AND enabled",
                vec![digest.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        Ok(Principal {
            id: column(&row, "id")?,
            role: Role::parse(&column::<String>(&row, "role")?)?,
        })
    }

    pub(super) async fn role(
        &self,
        db: &impl ConnectionTrait,
        principal: &Principal,
        required: Role,
    ) -> Result<(), Error> {
        if principal.role != required {
            return Err(Error::NotFound);
        }
        db.query_one_raw(self.statement(
            "SELECT id FROM @tower.operators WHERE id=$1 AND role=$2 AND enabled FOR SHARE",
            vec![principal.id.into(), required.name().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::NotFound)?;
        Ok(())
    }

    /// Advertise only bounded exact executable identities, not paths or remote commands.
    /// The host obtains them by probing its approved agents' local installations.
    /// # Errors
    /// Wrong/disabled role or invalid/duplicate/unbounded capabilities.
    pub async fn advertise(
        &self,
        principal: &Principal,
        identities: &[Identity],
    ) -> Result<(), Error> {
        if !valid_capabilities(identities) {
            return Err(Refusal::Invalid.into());
        }
        if principal.role != Role::Host {
            return Err(Error::NotFound);
        }
        let tx = self.transaction().await?;
        let changed = tx.execute_raw(self.statement("UPDATE @tower.operators SET capabilities=$2 WHERE id=$1 AND role='host' AND enabled", vec![principal.id.into(), encode(&identities)?.into()]))
            .await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::NotFound);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Read one enabled host's declared exact inventory before assigning a package.
    /// Declaration is a compatibility promise, not proof of a host's honesty.
    /// # Errors
    /// Unknown/disabled/wrong-role host or malformed stored capabilities.
    pub async fn host_supports(&self, host: Uuid, identity: &Identity) -> Result<bool, Error> {
        let row = self
            .db
            .query_one_raw(self.statement(
                "SELECT capabilities FROM @tower.operators WHERE id=$1 AND role='host' AND enabled",
                vec![host.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        let capabilities: Vec<Identity> = decode(&column::<Vec<u8>>(&row, "capabilities")?)?;
        if capabilities.is_empty() {
            return Ok(false);
        }
        if !valid_capabilities(&capabilities) {
            return Err(Error::Unavailable);
        }
        Ok(capabilities.contains(identity))
    }
}
