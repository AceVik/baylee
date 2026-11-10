//! `cards/artifacts/equipment/mv_2/specter_s_shroud.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn specter_s_shroud() -> CardIndex {
    card_index("dbc2b0f9-73e8-41b7-9d64-07f86fbc5d6c")
}

/// Specter's Shroud: "Equipped creature gets +1/+0. Whenever equipped
/// creature deals combat damage to a player, that player discards a card.
/// Equip {1}".
#[test]
fn specter_s_shroud_makes_the_damaged_player_discard() {
    let p1 = PlayerId::new(1);
    let mut r = rig_vs(
        specter_s_shroud(),
        llanowar_elves(),
        1,
        &[],
        &[llanowar_elves()],
        &[forest()],
    );
    r.equip(1);
    assert_eq!(pt(&r.engine, r.host), (2, 1), "+1/+0");
    assert!(
        in_hand(&r.engine, p1, forest()).is_some(),
        "they hold a card"
    );
    r.attack_with_host();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = r.engine.pending().clone()
    else {
        unreachable!("pass_until stops on the discard")
    };
    assert_eq!(player, p1, "the damaged player discards");
    assert_eq!(options.len(), 1, "from their one-card hand");
    r.engine
        .apply(p1, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(r.engine.state().players[1].life, 18, "2 combat damage");
    assert!(
        in_hand(&r.engine, p1, forest()).is_none(),
        "their hand is empty"
    );
    assert!(
        in_graveyard(&r.engine, p1, forest()).is_some(),
        "the discarded card is in their graveyard"
    );
}
