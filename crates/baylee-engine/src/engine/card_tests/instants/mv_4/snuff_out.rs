//! `cards/instants/mv_4/snuff_out.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "324824cb-f938-401c-b9b5-d8908b431ef0"
fn snuff_out() -> CardIndex {
    card_index("324824cb-f938-401c-b9b5-d8908b431ef0")
}

/// `Snuff Out` (`Coverage::Partial`):
/// "Destroy target nonblack creature. It can't be regenerated."
///
/// Verifies that casting `Snuff Out` destroys target nonblack creature and
/// cannot target a black creature.
#[test]
fn snuff_out_destroys_target_nonblack_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9603, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::DRUDGE_SKELETONS])
        .hand(0, &[snuff_out()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    let skel = on_battlefield(&engine, p1, index::DRUDGE_SKELETONS).expect("P1 Drudge Skeletons");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, snuff_out());
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![skel],
                    players: vec![],
                },
            )
            .is_err(),
        "Snuff Out cannot target a black creature"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none(),
        "Snuff Out destroys target nonblack creature"
    );
}
