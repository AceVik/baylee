#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::{ALERT_SHU_INFANTRY, DOOM_BLADE, TWIDDLE};

fn subject() -> CardIndex {
    card_index("602cce3e-7033-47d6-aec1-d755f234f270")
}

// Scenario: tapped other creature we control gains indestructible; an
// untapped friend, the source itself, and an opponent creature do not.
#[test]
fn adept_watershaper_protects_only_other_tapped_friends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                subject(),
                llanowar_elves(),
                ALERT_SHU_INFANTRY,
                island(),
                island(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), swamp(), swamp()])
        .hand(0, &[TWIDDLE, TWIDDLE])
        .hand(1, &[DOOM_BLADE])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let source = on_battlefield(&e, p0, subject()).unwrap();
    let tapped = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let idle = on_battlefield(&e, p0, ALERT_SHU_INFANTRY).unwrap();
    let theirs = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert!(!keywords(&e, tapped).contains(KeywordSet::INDESTRUCTIBLE));
    tap_mana_where(&mut e, p0, |id| id == tapped);
    assert!(is_tapped(&e, tapped));
    assert!(keywords(&e, tapped).contains(KeywordSet::INDESTRUCTIBLE));
    // Tap the source and the opposing Elf via actual spells; neither meets
    // the full "other tapped creatures you control" filter.
    for target in [source, theirs] {
        cast_from_hand(&mut e, p0, TWIDDLE);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
        pass_until(&mut e, |game| {
            matches!(game.pending(), Pending::YesNo { .. })
        });
        e.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    assert!(is_tapped(&e, source) && is_tapped(&e, theirs));
    for id in [source, idle, theirs] {
        assert!(!keywords(&e, id).contains(KeywordSet::INDESTRUCTIBLE));
    }
    assert!(keywords(&e, tapped).contains(KeywordSet::INDESTRUCTIBLE));
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, DOOM_BLADE);
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![tapped],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, llanowar_elves()).is_some());
    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));
    assert!(!is_tapped(&e, tapped));
    assert!(!keywords(&e, tapped).contains(KeywordSet::INDESTRUCTIBLE));
}
