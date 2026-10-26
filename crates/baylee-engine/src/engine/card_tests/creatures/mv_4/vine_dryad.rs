//! `cards/creatures/mv_4/vine_dryad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "990cd7de-66a2-4def-bf00-390b2682e995"
fn vine_dryad() -> CardIndex {
    card_index("990cd7de-66a2-4def-bf00-390b2682e995")
}

/// `Vine Dryad`:
/// "You may exile a green card from your hand rather than pay this spell's mana cost."
/// "Flash, Forestwalk"
#[test]
fn vine_dryad_pitches_a_green_card_and_has_flash_and_forestwalk() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9707, forest())
        .battlefield(1, &[index::GRIZZLY_BEARS, forest()])
        .hand(0, &[vine_dryad(), index::GRIZZLY_BEARS, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    // Walk to P1's main phase so casting Vine Dryad also exercises Flash.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let dryad_card = in_hand(&engine, p0, vine_dryad()).expect("Vine Dryad in hand");
    let green_pitch = in_hand(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears in hand");
    let red_card = in_hand(&engine, p0, lightning_bolt()).expect("Lightning Bolt in hand");

    engine
        .apply(p0, PlayerAction::CastSpell { card: dryad_card })
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected pitch ChooseCards, got {:?}", engine.pending());
    };
    assert!(options.contains(&green_pitch));
    assert!(!options.contains(&red_card));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![green_pitch],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(green_pitch).unwrap().zone,
        Zone::Exile
    );

    let dryad = on_battlefield(&engine, p0, vine_dryad()).expect("Vine Dryad");
    let kw = keywords(&engine, dryad);
    assert!(kw.contains(KeywordSet::FLASH));
    assert!(kw.contains(KeywordSet::FORESTWALK));

    // On P0's next turn, Vine Dryad has no summoning sickness and attacks unblockably via Forestwalk.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("defender's Bears");
    let blocks = attack_and_collect_blocks(&mut engine, dryad, p1);
    assert!(!blocks.iter().any(|option| option.blocker == bears));
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, dryad)],
                }
            )
            .is_err(),
        "the defending player's Forest makes the Dryad unblockable"
    );
    pass_until(&mut engine, |e| e.state().turn.step == Step::End);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "Vine Dryad is unblockable while defending player controls a Forest"
    );
}
