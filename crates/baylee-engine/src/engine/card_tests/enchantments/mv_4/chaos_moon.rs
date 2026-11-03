//! `cards/enchantments/mv_4/chaos_moon.rs`, played.
//!
//! Oracle:
//! - "At the beginning of each upkeep, count the number of permanents. If the number is odd, until end of turn, red creatures get +1/+1 and whenever a player taps a Mountain for mana, that player adds an additional {R}. If the number is even, until end of turn, red creatures get -1/-1 and if a player taps a Mountain for mana, that Mountain produces colorless mana instead of any other type."
//!
//! The tests put Chaos Moon on the battlefield before the first upkeep so
//! both parity branches and their end-of-turn expiry can be played directly.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "728e2960-1d85-4b3f-8b5b-915f9b9ffd2f"
fn chaos_moon() -> CardIndex {
    card_index("728e2960-1d85-4b3f-8b5b-915f9b9ffd2f")
}

#[track_caller]
fn tap_for_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId, land: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the basic land taps for mana");
}

#[test]
fn chaos_moon_odd_count_pumps_red_creatures_and_adds_red_for_every_player() {
    // Seven permanents: six under P0 and the opponent's Mountain.
    let mut engine = lk::game(
        SEED,
        &[
            chaos_moon(),
            index::HILL_GIANT,
            index::GRIZZLY_BEARS,
            mountain(),
            mountain(),
            forest(),
        ],
        &[mountain()],
        &[],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let mountains = all_on_battlefield(&engine, P0, mountain());
    let wood = lk::obj(&engine, P0, forest());
    let their_mountain = lk::obj(&engine, P1, mountain());

    assert_eq!(pt(&engine, giant), (4, 4), "odd count grants +1/+1");
    assert_eq!(
        pt(&engine, bears),
        (2, 2),
        "non-red creatures are unchanged"
    );

    tap_for_mana(&mut engine, P0, mountains[0]);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Red), 2);
    tap_for_mana(&mut engine, P0, wood);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Green), 1);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Red),
        2,
        "a Forest does not trigger the Mountain rider"
    );

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_for_mana(&mut engine, P1, their_mountain);
    assert_eq!(
        lk::pool_of(&engine, P1, ManaColor::Red),
        2,
        "the opponent also adds an additional red mana"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    assert_eq!(pt(&engine, giant), (3, 3), "the pump ended with P1's turn");
    tap_for_mana(&mut engine, P0, mountains[1]);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Red),
        1,
        "the additional mana also ended before the new trigger resolves"
    );
}

#[test]
fn chaos_moon_even_count_reduces_red_creatures_and_replaces_mountain_mana() {
    // Eight permanents: six under P0 and two under P1.
    let mut engine = lk::game(
        SEED,
        &[
            chaos_moon(),
            index::HILL_GIANT,
            index::GRIZZLY_BEARS,
            mountain(),
            mountain(),
            forest(),
        ],
        &[mountain(), forest()],
        &[],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let mountains = all_on_battlefield(&engine, P0, mountain());
    let wood = lk::obj(&engine, P0, forest());
    let their_mountain = lk::obj(&engine, P1, mountain());

    assert_eq!(pt(&engine, giant), (2, 2), "even count grants -1/-1");
    assert_eq!(
        pt(&engine, bears),
        (2, 2),
        "non-red creatures are unchanged"
    );

    tap_for_mana(&mut engine, P0, mountains[0]);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Colorless), 1);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Red), 0);
    tap_for_mana(&mut engine, P0, wood);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Green),
        1,
        "the replacement applies only to Mountains"
    );

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_for_mana(&mut engine, P1, their_mountain);
    assert_eq!(lk::pool_of(&engine, P1, ManaColor::Colorless), 1);
    assert_eq!(
        lk::pool_of(&engine, P1, ManaColor::Red),
        0,
        "the opponent's Mountain produces colorless instead of red"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    assert_eq!(pt(&engine, giant), (3, 3), "the -1/-1 ended with P1's turn");
    tap_for_mana(&mut engine, P0, mountains[1]);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Red), 1);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Colorless),
        0,
        "the mana replacement ended before the new trigger resolves"
    );
}

#[test]
fn chaos_moon_counts_permanents_when_its_upkeep_trigger_resolves() {
    // Five permanents at the beginning of upkeep. Lightning Bolt removes one
    // while the trigger is on the stack, leaving an even four on resolution.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                chaos_moon(),
                index::HILL_GIANT,
                index::HILL_GIANT,
                mountain(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    let giants = all_on_battlefield(&engine, P0, index::HILL_GIANT);
    let mountain = lk::obj(&engine, P0, mountain());
    let survivor = giants[1];
    assert_eq!(
        pt(&engine, survivor),
        (3, 3),
        "the trigger has not resolved"
    );

    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P1, index::LIGHTNING_BOLT, &[giants[0]], &[]);

    assert_eq!(
        pt(&engine, survivor),
        (2, 2),
        "the resolution-time count is even, so the survivor gets -1/-1"
    );
    tap_for_mana(&mut engine, P0, mountain);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Colorless), 1);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Red), 0);
}
