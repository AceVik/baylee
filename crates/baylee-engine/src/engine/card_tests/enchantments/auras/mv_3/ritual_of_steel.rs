//! `cards/enchantments/auras/mv_3/ritual_of_steel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ritual_of_steel() -> CardIndex {
    card_index("0af91b63-a874-4184-8f8f-3871429fd1ad")
}

/// Ritual of Steel (`{2}{W}`, `Coverage::Generated`): enchants a creature,
/// gives it +0/+2, and draws a card at the beginning of the next turn's upkeep.
#[test]
fn ritual_of_steel_pumps_toughness_and_draws_on_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), grizzly_bears()])
        .hand(0, &[ritual_of_steel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, ritual_of_steel());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (2, 4));
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == crate::turn::Step::Draw
            && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}

/// "Enchant creature" is either side's, the +0/+2 is for the enchanted
/// creature alone, and the card is drawn once, by the Aura's controller, at
/// the next turn's upkeep and not again.
#[test]
fn ritual_of_steel_pumps_only_the_enchanted_creature_and_draws_just_once() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[ritual_of_steel()],
        &[],
    );
    let mine = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let theirs = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, ritual_of_steel());
    let menu = aim_at(&mut engine, P0, mine);
    assert!(menu.contains(&mine) && menu.contains(&theirs));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mine), (2, 4));
    assert_eq!(pt(&engine, theirs), (2, 2), "only the enchanted creature");

    let p0_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let p1_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand + 1,
        "the Aura's controller drew at the next upkeep"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand + 1,
        "the opponent drew only its own draw-step card"
    );

    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand + 2,
        "only the normal draw at the following turn: once, not every upkeep"
    );
}
