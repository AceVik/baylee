//! `cards/creatures/mv_4/vine_dryad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "990cd7de-66a2-4def-bf00-390b2682e995"
fn vine_dryad() -> CardIndex {
    card_index("990cd7de-66a2-4def-bf00-390b2682e995")
}

/// `Vine Dryad` (`Coverage::Partial`):
/// "Flash, Forestwalk"
#[test]
fn vine_dryad_has_flash_and_forestwalk() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9707, forest())
        .battlefield(0, &[vine_dryad()])
        .battlefield(1, &[index::GRIZZLY_BEARS, forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let dryad = on_battlefield(&engine, p0, vine_dryad()).expect("Vine Dryad");
    let kw = keywords(&engine, dryad);
    assert!(kw.contains(KeywordSet::FLASH));
    assert!(kw.contains(KeywordSet::FORESTWALK));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dryad, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::End);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "Vine Dryad is unblockable while defending player controls a Forest"
    );
}
