//! `cards/sorceries/mv_4/cataclysm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn cataclysm() -> CardIndex {
    card_index("b0479ca4-e925-4aa7-a4fa-f7e01f899ac5")
}

/// Cataclysm (`b0479ca4-e925-4aa7-a4fa-f7e01f899ac5`) — {2}{W}{W} Sorcery (Partial):
/// "Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest."
#[test]
fn cataclysm_casts_and_resolves() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[cataclysm()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    cast_from_hand(&mut engine, P0, cataclysm());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, cataclysm()).is_some());
}
