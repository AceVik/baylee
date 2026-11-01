//! `cards/creatures/mv_5/scragnoth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{COUNTERSPELL, PRODIGAL_SORCERER};

fn scragnoth() -> CardIndex {
    card_index("9e66f2d6-cc51-4747-914d-2c04e439da2b")
}

/// `Scragnoth` ({4}{G}, 3/4 Creature — Beast, `Coverage::Implemented`):
/// `This spell can't be countered.`
/// `Protection from blue`
#[test]
fn scragnoth_cannot_be_countered_and_has_protection_from_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[island(), island(), PRODIGAL_SORCERER])
        .hand(0, &[scragnoth()])
        .hand(1, &[COUNTERSPELL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, scragnoth());
    let spell_id = engine.state().zones.list(ZoneLocation::Stack)[0];

    // Opponent responds with Counterspell targeting Scragnoth.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, COUNTERSPELL);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![spell_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Scragnoth was not countered and entered the battlefield.
    let beast = on_battlefield(&engine, p0, scragnoth()).unwrap();
    let ch = engine.state().object(beast).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(4)));

    // Protection from blue prevents blue Prodigal Sorcerer from targeting Scragnoth.
    reach_their_main_phase(&mut engine, p1);
    activate(&mut engine, p1, PRODIGAL_SORCERER, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(!options.contains(&beast));
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ChooseTargets {
                    objects: vec![beast],
                    players: vec![],
                },
            )
            .is_err()
    );
}

#[test]
fn protection_excludes_blue_blockers_but_allows_green_blockers() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[scragnoth()],
        &[index::AIR_ELEMENTAL, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let attacker = on_battlefield(&e, p0, scragnoth()).unwrap();
    let blue = on_battlefield(&e, p1, index::AIR_ELEMENTAL).unwrap();
    let green = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    let blocks = lk::attack(&mut e, &[attacker]);
    assert!(!lk::may_block(&blocks, blue, attacker));
    assert!(lk::may_block(&blocks, green, attacker));
    lk::block(&mut e, &[(green, attacker)]);
}
