//! `cards/creatures/mv_4/leonin_abunas.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leonin Abunas prints one static — "Artifacts you control have hexproof"
/// (CR 702.11b) — and the whole card is a claim about *targeting*, so the proof
/// is read out of the opponent's own hand rather than off the layer projection.
/// p1's first Vindicate must offer the Abunas itself and the Llanowar Elves
/// beside it, permanents p0 also controls and neither of which has hexproof,
/// while refusing the Sol Ring in the same row; and once that same Vindicate has
/// killed the Abunas, the second copy from the same hand finds the Sol Ring on
/// the menu — the identical board minus one card, which is what tells the
/// printed sentence from a target menu that was short for some other reason.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn leonin_abunas_hexproofs_the_artifacts_you_control_from_the_opponents_spells() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[leonin_abunas()])
        .battlefield(
            1,
            &[
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .hand(1, &[vindicate(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // The card arrives the way the card arrives, with everything it is about
    // standing beside it: an artifact p0 controls, an artifact p1 controls, and
    // a creature p0 controls that is no artifact at all.
    cast_from_hand(&mut engine, p0, leonin_abunas());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, leonin_abunas()).is_some()
    });
    let abunas = on_battlefield(&engine, p0, leonin_abunas()).expect("the Abunas resolved");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let their_ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    assert!(
        keywords(&engine, ring).contains(KeywordSet::HEXPROOF),
        "the printed static reaches the artifact on p0's battlefield"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::HEXPROOF),
        "\"artifacts you control\" is not \"permanents you control\""
    );
    assert!(
        !keywords(&engine, their_ring).contains(KeywordSet::HEXPROOF),
        "\"you control\" is not \"the table\": the same card on the other side \
         gets nothing from p0's Abunas"
    );

    // Over to the opponent. Three Plains and three Swamps pay for both
    // Vindicates — {1}{W}{B} twice is two white, two black and two more of
    // anything — and CR 500.5 keeps whatever the first does not spend in the
    // pool for the second, since the whole scenario stays in one main phase.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    let menu = pass_until_targets(&mut engine, p1);
    assert!(
        menu.contains(&abunas) && menu.contains(&elf),
        "\"target permanent\" reaches the Abunas itself and the Elves beside \
         it, both of which p0 controls with no hexproof on them: {menu:?}"
    );
    assert!(
        menu.contains(&their_ring),
        "and it reaches the artifact on p1's own side of the table, so the \
         refusal below is not a menu about artifacts in general: {menu:?}"
    );
    assert!(
        !menu.contains(&ring),
        "CR 702.11b: an artifact p0 controls cannot be the target of a spell an \
         opponent controls, so it is not on the menu at all: {menu:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![abunas],
            },
        )
        .expect("the Abunas was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, leonin_abunas()).is_some(),
        "the first Vindicate resolved against the Abunas and it died"
    );

    // The same board minus one card, and the second Vindicate is cast off the
    // mana the first left floating.
    cast_from_hand(&mut engine, p1, vindicate());
    let after = pass_until_targets(&mut engine, p1);
    assert!(
        after.contains(&ring),
        "with the Abunas gone the Sol Ring is a permanent like any other, which \
         is what says the refusal above came from the printed static: {after:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the Sol Ring was on the menu once nothing granted it hexproof");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none()
            && in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and the second Vindicate destroyed it, so the artifact was not merely \
         named: it left the battlefield for its owner's graveyard"
    );
}
