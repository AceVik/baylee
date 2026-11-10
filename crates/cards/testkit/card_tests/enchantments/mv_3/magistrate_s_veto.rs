//! `cards/enchantments/mv_3/magistrate_s_veto.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "f50f8cac-35bf-4d4d-88c0-1c0c581f862c"
fn magistrate_s_veto() -> CardIndex {
    card_index("f50f8cac-35bf-4d4d-88c0-1c0c581f862c")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// `Magistrate's Veto` (`Coverage::Generated`):
/// "White creatures and blue creatures can't block."
///
/// Verifies that white and blue creatures gain `KeywordSet::CANT_BLOCK` and cannot
/// block while green creatures remain able to block.
#[test]
fn magistrates_veto_prevents_white_and_blue_creatures_from_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9414, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), grizzly_bears()])
        .battlefield(
            1,
            &[
                savannah_lions(),
                merfolk_of_the_pearl_trident(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[magistrate_s_veto()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, magistrate_s_veto());
    pass_until(&mut engine, stack_is_empty);

    let white_c = on_battlefield(&engine, p1, savannah_lions()).expect("white creature");
    let blue_c =
        on_battlefield(&engine, p1, merfolk_of_the_pearl_trident()).expect("blue creature");
    let green_c = on_battlefield(&engine, p1, grizzly_bears()).expect("green creature");

    assert!(keywords(&engine, white_c).contains(KeywordSet::CANT_BLOCK));
    assert!(keywords(&engine, blue_c).contains(KeywordSet::CANT_BLOCK));
    assert!(!keywords(&engine, green_c).contains(KeywordSet::CANT_BLOCK));

    let attacker = on_battlefield(&engine, p0, grizzly_bears()).expect("P0 attacker");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(white_c, attacker)],
                },
            )
            .is_err(),
        "white creature cannot block under Magistrate's Veto"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blue_c, attacker)],
                },
            )
            .is_err(),
        "blue creature cannot block under Magistrate's Veto"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(green_c, attacker)],
            },
        )
        .expect("green creature can still block");
}
