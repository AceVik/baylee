//! `cards/artifacts/equipment/mv_3/champion_s_helm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn champion_s_helm() -> CardIndex {
    card_index("c01aafb8-3da5-4eb1-8731-2a223e747d63")
}

/// Champion's Helm: "Equipped creature gets +2/+2. As long as equipped
/// creature is legendary, it has hexproof. Equip {1}".
#[test]
fn champion_s_helm_gives_a_legendary_host_hexproof() {
    equipped_gets(
        champion_s_helm(),
        1,
        jasmine_boreal(),
        (2, 2),
        KeywordSet::HEXPROOF,
    );
}

#[test]
fn champion_s_helm_gives_a_nonlegendary_host_only_the_bonus() {
    equipped_stays_without(
        champion_s_helm(),
        1,
        llanowar_elves(),
        (2, 2),
        KeywordSet::HEXPROOF,
    );
}
