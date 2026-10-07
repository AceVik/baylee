//! `cards/creatures/mv_4/ojer_axonil_deepest_might.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ojer Axonil, Deepest Might` // `Temple of Power` (`Coverage::Partial`):
/// "Trample. If a red source you control would deal an amount of noncombat damage less than
/// `Ojer Axonil`'s power to an opponent, that source deals damage equal to `Ojer Axonil`'s power instead.
/// When `Ojer Axonil` dies, return it to the battlefield tapped and transformed under its owner's
/// control. // `{{T}}`: Add `{{R}}`. `{{2}}{{R}}`, `{{T}}`: Transform this land. Activate only if red sources
/// you controlled dealt 4 or more noncombat damage this turn and only as a sorcery."
///
/// Under `Coverage::Partial`, the noncombat damage replacement and transform-back are omitted,
/// leaving `KeywordSet::TRAMPLE` on a 4/4 God and the dies-trigger `Effect::ExileSelfReturnAsFace`.
/// The test verifies that `Ojer Axonil` starts with trample, is destroyed by an opponent's
/// `heroes_downfall()`, and returns to the battlefield transformed as the land `Temple of Power` on face 1.
#[test]
fn ojer_axonil_deepest_might_dies_and_returns_as_temple_of_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(309, swamp())
        .battlefield(0, &[ojer_axonil_deepest_might()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[heroes_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p0, ojer_axonil_deepest_might())
        .expect("Ojer Axonil on battlefield");
    assert_eq!(pt(&engine, god), (4, 4), "starts as a 4/4 God");
    assert!(
        keywords(&engine, god).contains(KeywordSet::TRAMPLE),
        "Ojer Axonil has trample"
    );

    // Advance to p1's main phase and cast Hero's Downfall targeting Ojer Axonil.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, heroes_downfall());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![god],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let temple = on_battlefield(&engine, p0, ojer_axonil_deepest_might())
        .expect("returned to battlefield transformed");
    assert_eq!(
        engine.state().object(temple).map(|o| o.face_index),
        Some(1),
        "returned as face 1 (Temple of Power)"
    );

    let t = types(&engine, temple);
    assert!(t.contains(TypeSet::LAND), "Temple of Power is a land");
    assert!(
        !t.contains(TypeSet::CREATURE),
        "Temple of Power is not a creature"
    );
    assert!(
        in_graveyard(&engine, p0, ojer_axonil_deepest_might()).is_none(),
        "the card is on the battlefield and not in the graveyard"
    );
}
