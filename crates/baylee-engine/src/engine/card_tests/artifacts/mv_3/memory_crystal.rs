//! `cards/artifacts/mv_3/memory_crystal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn memory_crystal() -> CardIndex {
    card_index("f5fd3141-6fd9-4491-9f29-7fd81209bd57")
}

fn capsize() -> CardIndex {
    card_index("77637eff-2963-4402-88f3-ca346f762fc8")
}

fn mind_games() -> CardIndex {
    card_index("ea43372f-b0a7-4d1b-be3e-a4a738b06d59")
}

fn the_price(engine: &Engine<RegistryLookup>) -> baylee_core::mana::ManaCost {
    match engine.pending() {
        Pending::YesNo {
            prompt: YesNoPrompt::Buyback { cost, .. },
            ..
        } => *cost,
        other => panic!("expected the buyback question, got {other:?}"),
    }
}

/// `crystals` Memory Crystals on seat `owner`'s side, and `spell` in p0's
/// hand over `lands` Islands. p0's main phase.
fn board(owner: usize, crystals: usize, spell: CardIndex, lands: usize) -> Engine<RegistryLookup> {
    let mut mine = vec![island(); lands];
    let mut theirs = vec![forest()];
    if owner == 0 {
        mine.extend(vec![memory_crystal(); crystals]);
    } else {
        theirs.extend(vec![memory_crystal(); crystals]);
    }
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &mine)
        .battlefield(1, &theirs)
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// "Buyback costs cost {2} less": Capsize's {3} is {1}, and the whole cast
/// is the four Islands that pay {1}{U}{U} and {1}.
#[test]
fn memory_crystal_takes_two_off_a_buyback_cost() {
    let mut engine = board(0, 1, capsize(), 4);
    let forest_id = on_battlefield(&engine, P1, forest()).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    assert_eq!(
        the_price(&engine),
        baylee_core::mana::ManaCost::parse("{1}")
    );
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, forest_id);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_bought_back(&engine, P0, capsize());
}

/// It takes generic mana alone (CR 118.7a): Mind Games' {2}{U} is {U}.
#[test]
fn memory_crystal_leaves_a_coloured_symbol() {
    let mut engine = board(0, 1, mind_games(), 2);
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, mind_games());
    assert_eq!(
        the_price(&engine),
        baylee_core::mana::ManaCost::parse("{U}")
    );
}

/// It says nothing of whose spell: an opponent's Crystal helps p0 as well.
#[test]
fn memory_crystal_reaches_every_players_buyback() {
    let mut engine = board(1, 1, capsize(), 4);
    cast_from_hand(&mut engine, P0, capsize());
    assert_eq!(
        the_price(&engine),
        baylee_core::mana::ManaCost::parse("{1}")
    );
}

/// Two Crystals take four, and a cost of {3} stops at nothing (CR 118.7),
/// with nothing taken off the spell's own {1}{U}{U}.
#[test]
fn two_memory_crystals_stop_at_nothing_and_touch_only_the_buyback() {
    let mut engine = board(0, 2, capsize(), 3);
    let forest_id = on_battlefield(&engine, P1, forest()).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    assert_eq!(the_price(&engine), baylee_core::mana::ManaCost::ZERO);
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, forest_id);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_bought_back(&engine, P0, capsize());
}

/// A spell whose buyback is not paid costs what it prints.
#[test]
fn memory_crystal_does_not_reduce_a_spell_cast_without_its_buyback() {
    let mut engine = board(0, 1, capsize(), 2);
    let spell = in_hand(&engine, P0, capsize()).unwrap();
    tap_all_mana(&mut engine, P0);
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "two Islands do not pay {{1}}{{U}}{{U}}"
    );
}
