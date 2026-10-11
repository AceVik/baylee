//! `cards/lands/spheres/mirrex.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mirrex: "{T}: Add {C}." / "{T}: Add one mana of any color. Activate only if this land entered this turn." / "{3}, {T}: Create a 1/1 colorless Phyrexian Mite..."
/// ability is written and played in `mirrex_enters_untapped_adds_any_color_only_on_entry_turn`.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn mirrex_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(327, forest()).hand(0, &[mirrex()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrex());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, mirrex(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Mirrex prints `{{T}}: Add {{C}}.`,
/// `{{T}}: Add one mana of any color. Activate only if this land entered this turn.`, and
/// `{{3}}, {{T}}: Create a 1/1 colorless Phyrexian Mite artifact creature token...`
/// This test plays Mirrex untapped, verifies that floating `{{3}}` with `tap_mana_except` does
/// on the turn it entered, and advances to the next turn where ability 1 is no longer offered and
/// ability 0 taps for colorless mana.
#[test]
fn mirrex_enters_untapped_adds_any_color_only_on_entry_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[mirrex()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrex());
    assert!(!entered_tapped(&engine, land), "Mirrex enters untapped");

    // Float {3} mana keeping Mirrex untapped.
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three green mana floating to afford potential costs"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };

    assert!(
        legal.abilities.contains(&(land, 2)),
        "the token ability is available with three mana"
    );

    // Both ability 0 ({C}) and ability 1 (any color on entry turn) are available.
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 0),
        "ability 0 is offered"
    );
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 1),
        "ability 1 is offered on the turn Mirrex entered"
    );

    // Activate ability 1 to produce blue mana.
    activate(&mut engine, p0, mirrex(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "offers all 5 colors");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choosing blue mana is legal");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "adds one blue mana"
    );
    assert!(is_tapped(&engine, land), "Mirrex tapped for ability 1");

    // On the following turn, Mirrex did not enter this turn and ability 1 is withheld.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "Mirrex untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 1),
        "ability 1 is withheld when Mirrex did not enter this turn"
    );

    // Ability 0 taps for colorless mana.
    activate(&mut engine, p0, mirrex(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "ability 0 adds {{C}}"
    );
    assert!(is_tapped(&engine, land));
}

#[test]
fn mirrex_mites_are_artifacts_that_cannot_block_and_toxic_is_combat_only() {
    use crate::engine::card_tests::token_helpers::{attack, named};
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MIRREX, index::DOUBLING_SEASON])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    activate(&mut e, p0, index::MIRREX, 2);
    pass_until(&mut e, stack_is_empty);
    let mites = named(&e, "Phyrexian Mite");
    assert_eq!(mites.len(), 2);
    let chars = e.state().object(mites[0]).unwrap().characteristics();
    assert!(chars.types.contains(baylee_core::types::TypeSet::ARTIFACT));
    crate::engine::card_tests::history_helpers::damage_player(&mut e, mites[0], p1, 1, false);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].poison, 0);
    attack(&mut e, mites[0]);
    assert_eq!(e.state().players[1].poison, 1);
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, baylee_core::ids::Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!blockers.iter().any(|o| mites.contains(&o.blocker)));
}
