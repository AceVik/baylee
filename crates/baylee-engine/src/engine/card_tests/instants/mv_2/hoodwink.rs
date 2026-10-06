//! `cards/instants/mv_2/hoodwink.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hoodwink — {1}{U} instant: "Return target artifact, enchantment, or land to
/// its owner's hand."
///
/// The filter is three types wide, so the board carries one permanent of each
/// legible kind across the table — a land and an artifact — plus a creature
/// under the caster's own control. The Elf is the counter-half: it is a
/// permanent a "target permanent" reading would have offered, and this card
/// must not, which no comparison of `CardDef` fields can separate from a filter
/// that quietly lost its type list. Aiming the spell at the opponent's Forest
/// is what reads the rest of the sentence: the card has to reach the hand of
/// the seat that *owns* it and not the seat that cast the spell, and the
/// artifact beside it and the caster's own board are the controls that exactly
/// one card moved.
#[test]
fn hoodwink_bounces_an_opponents_land_to_its_owners_hand_and_never_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[forest(), quiet_artifact()])
        .hand(0, &[hoodwink()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let my_land = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");

    // {1}{U} off two Islands; the Elf is tapped by the helper too, which is
    // harmless here — nothing below counts the pool or the Elves' status.
    cast_from_hand(&mut engine, p0, hoodwink());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster is the seat that aims the spell");
    assert!(
        options.contains(&their_land) && options.contains(&their_rock),
        "an opponent's land and an opponent's artifact are both \"target \
         artifact, enchantment, or land\": {options:?}"
    );
    assert!(
        options.contains(&my_land),
        "and so is the caster's own land — the filter names no controller: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is none of the three types the card prints, which is what \
         tells this filter from \"target permanent\": {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "the two Islands, their Forest and their artifact are the whole menu; \
         the Elf is the only permanent on the table the card declines: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the Forest was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the targeted land left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, forest()).is_some(),
        "\"to its owner's hand\": the Forest goes back to the seat that owns \
         it, not to the seat that cast the spell"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_none(),
        "and not to the caster's hand, which is where a bounce misread as \
         \"return it to your hand\" would have put it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "one target moved one card: the artifact beside it never moved"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature the spell could not name is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, hoodwink()).is_some(),
        "an instant that resolves goes to its caster's graveyard"
    );
}
