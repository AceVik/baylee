//! `cards/instants/mv_3/battle_cry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::partials_w2c_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn battle_cry() -> CardIndex {
    card_index("b92afdba-5aea-42c8-bdd8-6ba5ff6cba4f")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn two_headed_giant_of_foriys() -> CardIndex {
    card_index("38aa31bd-7145-43b9-9409-463d9ad6cd69")
}

#[track_caller]
fn cast_battle_cry(engine: &mut Engine<RegistryLookup>) {
    tap_all_mana_but(engine, P0, Some(llanowar_elves()));
    cast_with_floating(engine, P0, battle_cry());
    pass_until(engine, stack_is_empty);
}

/// Battle Cry (b92afdba-5aea-42c8-bdd8-6ba5ff6cba4f) — {2}{W} Instant:
/// - "Untap all white creatures you control."
/// - "Whenever a creature blocks this turn, it gets +0/+1 until end of turn."
#[test]
fn battle_cry_untaps_all_white_creatures_you_control_and_leaves_others_tapped() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                savannah_lions(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[savannah_lions()])
        .hand(0, &[battle_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_lions = on_battlefield(&engine, P0, savannah_lions()).expect("my Savannah Lions");
    let my_elf = on_battlefield(&engine, P0, llanowar_elves()).expect("my Llanowar Elves");
    let their_lions = on_battlefield(&engine, P1, savannah_lions()).expect("their Savannah Lions");

    for id in [my_lions, my_elf, their_lions] {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }

    cast_battle_cry(&mut engine);

    assert!(
        !is_tapped(&engine, my_lions),
        "white creature you control is untapped"
    );
    assert!(
        is_tapped(&engine, my_elf),
        "non-white creature you control stays tapped"
    );
    assert!(
        is_tapped(&engine, their_lions),
        "opponent's white creature stays tapped"
    );
}

#[test]
fn battle_cry_pumps_an_opposing_blocker_before_it_survives_real_combat_damage() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), grizzly_bears()])
        .hand(0, &[battle_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let attacker = on_battlefield(&engine, P0, llanowar_elves()).expect("attacker");
    let blocker = on_battlefield(&engine, P1, llanowar_elves()).expect("blocker");
    let bystander = on_battlefield(&engine, P1, grizzly_bears()).expect("nonblocker");
    cast_battle_cry(&mut engine);

    kit::attack(&mut engine, P0, &[attacker], P1);
    kit::block(&mut engine, P1, &[(blocker, attacker)]);
    kit::settle(&mut engine, P0, &mut kit::Answers::new());

    assert_eq!(
        kit::power_toughness(&engine, blocker),
        (1, 2),
        "the opposing blocker gets +0/+1, not +1/+1"
    );
    assert_eq!(kit::power_toughness(&engine, attacker), (1, 1));
    assert_eq!(
        kit::power_toughness(&engine, bystander),
        (2, 2),
        "a creature that did not block is unchanged"
    );

    kit::drive(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        kit::on_the_table(&engine, blocker),
        "the 1/2 blocker survives one combat damage"
    );
    assert_eq!(kit::damage_on(&engine, blocker), 1);
    assert_eq!(kit::zone_of(&engine, attacker), Zone::Graveyard);

    kit::walk_to_next_own_main(&mut engine, P0);
    assert_eq!(
        kit::power_toughness(&engine, blocker),
        (1, 1),
        "the bonus and marked damage end with the turn"
    );
}

#[test]
fn battle_cry_triggers_once_for_a_creature_blocking_two_attackers() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[two_headed_giant_of_foriys()])
        .hand(0, &[battle_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let attackers = kit::objects_of(&engine, P0, llanowar_elves());
    assert_eq!(attackers.len(), 2);
    let blocker = on_battlefield(&engine, P1, two_headed_giant_of_foriys()).expect("blocker");
    cast_battle_cry(&mut engine);

    kit::attack(&mut engine, P0, &attackers, P1);
    kit::block(
        &mut engine,
        P1,
        &[(blocker, attackers[0]), (blocker, attackers[1])],
    );
    kit::settle(&mut engine, P0, &mut kit::Answers::new());

    assert_eq!(
        kit::power_toughness(&engine, blocker),
        (4, 5),
        "one blocking declaration is one event, not one event per blocked pair"
    );
}
