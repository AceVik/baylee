//! `cards/instants/mv_2/samite_ministration.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn samite_ministration() -> CardIndex {
    card_index("af669df1-3526-46ca-b90c-19b112b7ba44")
}

fn raging_goblin() -> CardIndex {
    card_index("30997b43-fc13-41d3-8064-1ccc2cb6fd2b")
}

/// P0 casts Samite Ministration in the opponent's first main phase, chooses
/// `source_card`, and the opponent then attacks with `attack_with`.
#[track_caller]
fn ministration_then_attack(
    source_card: CardIndex,
    attack_with: &[CardIndex],
) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(
            1,
            &[
                grizzly_bears(),
                gray_ogre(),
                raging_goblin(),
                scathe_zombies(),
            ],
        )
        .hand(0, &[samite_ministration()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let source_object = on_battlefield(&engine, p1, source_card).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, samite_ministration());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    let source = options
        .into_iter()
        .find(|s| s.object == source_object)
        .expect("the creature is a damage source option");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let attackers = attack_with
        .iter()
        .map(|card| {
            (
                on_battlefield(&engine, p1, *card).unwrap(),
                Defender::Player(p0),
            )
        })
        .collect();
    engine
        .apply(p1, PlayerAction::DeclareAttackers { attackers })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    engine
}

#[test]
fn red_chosen_source_gains_life_while_another_sources_damage_gets_through() {
    let engine = ministration_then_attack(gray_ogre(), &[gray_ogre(), raging_goblin()]);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "the Ogre's 2 is prevented and gained as life; the Goblin deals 1"
    );
}

#[test]
fn black_chosen_source_also_gains_the_prevented_damage_as_life() {
    let engine = ministration_then_attack(scathe_zombies(), &[scathe_zombies()]);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "the Zombies' 2 is prevented and gained as life"
    );
}

#[test]
fn neither_red_nor_black_chosen_source_only_prevents_damage() {
    let engine = ministration_then_attack(grizzly_bears(), &[grizzly_bears()]);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Bears' 2 is prevented without gaining life"
    );
}
