//! `cards/instants/mv_1/abjure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index::{GRIZZLY_BEARS, MERFOLK_OF_THE_PEARL_TRIDENT};

fn abjure() -> CardIndex {
    card_index("dd97e457-07eb-4918-b780-ae7ff42a37e7")
}

/// `Abjure` ({U}, Instant) sacrifices a blue permanent as an additional cost
/// to counter target spell.
#[test]
fn abjure_sacrifices_blue_permanent_to_counter_target_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[island(), MERFOLK_OF_THE_PEARL_TRIDENT])
        .hand(0, &[GRIZZLY_BEARS])
        .hand(1, &[abjure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let merfolk = on_battlefield(&engine, p1, MERFOLK_OF_THE_PEARL_TRIDENT).unwrap();
    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    let spell_id = on_stack(&engine, GRIZZLY_BEARS).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, p1, abjure());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![spell_id],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![merfolk],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, MERFOLK_OF_THE_PEARL_TRIDENT).is_some());
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
}

/// The additional cost is a *blue permanent you control*: a colorless Island
/// is not blue, so with nothing blue on the board Abjure is not castable even
/// with {U} floating, and with the opponent's Merfolk the only blue thing out
/// it is still not castable. Given a blue permanent of its own, it is.
#[test]
fn abjure_needs_a_blue_permanent_of_its_own_to_be_cast() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest(), MERFOLK_OF_THE_PEARL_TRIDENT])
        .battlefield(1, &[island()])
        .hand(0, &[GRIZZLY_BEARS])
        .hand(1, &[abjure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    assert_eq!(lk::pool_total(&engine, p1), 1, "{{U}} is floating");
    assert!(
        !lk::castable(&engine, p1, abjure()),
        "an Island is not blue, and the other side's Merfolk is not ours to sacrifice"
    );
}

/// The sacrifice question lists the caster's blue permanents and nothing else:
/// not the colorless Island, not the opponent's Merfolk.
#[test]
fn abjure_offers_only_its_casters_blue_permanents_to_sacrifice() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest(), MERFOLK_OF_THE_PEARL_TRIDENT])
        .battlefield(1, &[island(), MERFOLK_OF_THE_PEARL_TRIDENT])
        .hand(0, &[GRIZZLY_BEARS])
        .hand(1, &[abjure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, MERFOLK_OF_THE_PEARL_TRIDENT).unwrap();
    let ours = on_battlefield(&engine, p0, MERFOLK_OF_THE_PEARL_TRIDENT).unwrap();
    let island_id = on_battlefield(&engine, p1, island()).unwrap();

    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    let spell_id = on_stack(&engine, GRIZZLY_BEARS).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    assert!(lk::castable(&engine, p1, abjure()));
    cast_with_floating(&mut engine, p1, abjure());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![spell_id],
                players: vec![],
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the sacrifice question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1);
    assert_eq!(options, vec![theirs], "exactly the caster's blue permanent");
    assert!(!options.contains(&ours) && !options.contains(&island_id));
}
