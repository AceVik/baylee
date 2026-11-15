//! Gateway selection: compatible installed artifacts, never remote executable paths.
use baylee_core::preset::LoopPolicy;
use baylee_protocol::execution::{Identity, Variant};

#[cfg(test)]
mod tests;

pub(super) fn native(variant: Variant) -> Identity {
    Identity::native(
        variant,
        baylee_build::short(),
        baylee_build::COMMIT,
        u32::from(baylee_core::preset::AIProfile::GENERATION),
    )
}

pub(crate) const fn standard_variant(policy: LoopPolicy) -> Variant {
    match policy {
        LoopPolicy::Off => Variant::Standard,
        LoopPolicy::RunOnceThenBreak | LoopPolicy::CompRulesDraw => Variant::StandardLoops,
    }
}

impl super::Agents {
    /// Least busy installed exact artifact. A cohort can never fall back to this release.
    pub(crate) fn pick_exact(&self, identity: &Identity) -> Option<(String, Identity)> {
        self.connected
            .iter()
            .filter(|(_, agent)| agent.has_room() && agent.engines.contains(identity))
            .min_by(|(a_id, a), (b_id, b)| {
                a.games
                    .len()
                    .cmp(&b.games.len())
                    .then_with(|| a_id.cmp(b_id))
            })
            .map(|(id, _)| (id.clone(), identity.clone()))
    }

    /// Installed compatibility survives full agents; availability is a separate subset.
    pub(crate) fn tower_inventory(&self) -> Option<TowerInventory> {
        let mut installed = Vec::new();
        let mut available = Vec::new();
        for agent in self.connected.values() {
            for identity in &agent.engines {
                if identity.variant != Variant::Tower {
                    continue;
                }
                if !installed.contains(identity) {
                    installed.push(identity.clone());
                    if installed.len() > baylee_protocol::execution::MAX_CAPABILITIES {
                        return None;
                    }
                }
                if agent.has_room() && !available.contains(identity) {
                    available.push(identity.clone());
                }
            }
        }
        installed.sort_by_key(Identity::to_metadata);
        available.sort_by_key(Identity::to_metadata);
        (installed.is_empty() || baylee_protocol::execution::valid_capabilities(&installed))
            .then_some(TowerInventory {
                installed,
                available,
            })
    }

    /// Least busy compatible agent, deterministic tie break, and its exact artifact.
    /// Retained certified v1 challenges freeze a build; ordinary games use this release.
    pub(crate) fn pick_variant(
        &self,
        variant: Variant,
        executor: Option<&str>,
    ) -> Option<(String, Identity)> {
        let expected = native(variant);
        self.connected
            .iter()
            .filter(|(_, agent)| agent.has_room())
            .filter_map(|(id, agent)| {
                let engine = agent.engines.iter().find(|identity| {
                    if let Some(build) = executor {
                        identity.valid()
                            && identity.build == build
                            && identity.variant == expected.variant
                            && identity.features == expected.features
                            && identity.rules == expected.rules
                            && identity.ai == expected.ai
                    } else {
                        **identity == expected
                    }
                })?;
                Some((id, agent, engine))
            })
            .min_by(|(id_a, a, _), (id_b, b, _)| {
                a.games
                    .len()
                    .cmp(&b.games.len())
                    .then_with(|| id_a.cmp(id_b))
            })
            .map(|(id, _, engine)| (id.clone(), engine.clone()))
    }
}

pub(crate) struct TowerInventory {
    pub installed: Vec<Identity>,
    pub available: Vec<Identity>,
}

/// Every field must agree, including the legacy build used by record/challenge readers.
pub(crate) fn matches(
    expected: Option<&Identity>,
    hello: &baylee_protocol::v1::EngineHello,
) -> bool {
    expected.is_some_and(|expected| {
        hello.build == expected.build
            && Identity::from_metadata(&hello.execution).as_ref() == Some(expected)
    })
}
