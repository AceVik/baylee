//! Serrated Arrows — `{4}` Artifact (Homelands)
//!
//! This artifact enters with three arrowhead counters on it.
//! At the beginning of your upkeep, if there are no arrowhead counters on this artifact, sacrifice it.
//! `{T}`, Remove an arrowhead counter from this artifact: Put a -1/-1 counter on target creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn serrated_arrows() -> CardIndex {
    card_index("5fa4607a-6731-4599-80f3-19d6097b4b28")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn shoot(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    arrows: ObjectId,
    target: ObjectId,
) {
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source: arrows,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

#[test]
fn serrated_arrows_enters_with_three_counters_puts_m1m1_counters_and_sacrifices_at_zero() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[serrated_arrows()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let arrows_card = in_hand(&engine, p0, serrated_arrows()).unwrap();
    let bears = all_on_battlefield(&engine, p1, grizzly_bears());

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: arrows_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let arrows = on_battlefield(&engine, p0, serrated_arrows()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(arrows)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ARROWHEAD),
        3,
        "Serrated Arrows entered with 3 arrowhead counters"
    );

    // Activate once to put a -1/-1 counter on the first Grizzly Bears (3 -> 2 arrowhead counters).
    shoot(&mut engine, p0, arrows, bears[0]);
    assert_eq!(pt(&engine, bears[0]), (1, 1));

    // Next turn: remove the 2nd arrowhead counter to kill the first Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    shoot(&mut engine, p0, arrows, bears[0]);
    assert_eq!(
        engine.state().object(bears[0]).unwrap().zone,
        Zone::Graveyard
    );

    // Next turn: remove the 3rd (last) arrowhead counter onto the second Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    shoot(&mut engine, p0, arrows, bears[1]);
    assert_eq!(
        engine
            .state()
            .object(arrows)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ARROWHEAD),
        0
    );

    // Advance to p0's next upkeep and resolve the sacrifice trigger: Serrated Arrows is sacrificed!
    // The opponent's upkeep is not yours: it is still there in their main phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().object(arrows).unwrap().zone,
        Zone::Battlefield,
        "no sacrifice at the beginning of the opponent's upkeep"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().object(arrows).unwrap().zone,
        Zone::Graveyard,
        "Serrated Arrows sacrificed itself at upkeep with 0 arrowhead counters"
    );
}
