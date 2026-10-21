//! Rysorian Badger — `{2}{G}` 2/2 Creature — Badger (Homelands)
//!
//! Whenever this creature attacks and isn't blocked, you may exile up to two target creature cards
//! from defending player's graveyard. If you do, you gain 1 life for each card exiled this way and
//! this creature assigns no combat damage this turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rysorian_badger() -> CardIndex {
    card_index("cf9fc317-3184-4999-84ef-945a8842096e")
}

#[test]
fn rysorian_badger_exiles_defending_graveyard_creatures_gains_life_and_assigns_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[rysorian_badger(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put Grizzly Bears into p1's graveyard with Lightning Bolt.
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let bear_bf = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_bf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let badger = on_battlefield(&engine, p0, rysorian_badger()).unwrap();
    let gy_bear = in_graveyard(&engine, p1, grizzly_bears()).unwrap();

    // Attack with Rysorian Badger unblocked.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(badger, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Choose target creature card in p1's graveyard for the trigger, then accept MayDo.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().object(gy_bear).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().players[0].life, 21);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "Rysorian Badger assigned no combat damage this turn"
    );
}
