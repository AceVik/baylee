//! `cards/enchantments/mv_3/heroism.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn heroism_card() -> CardIndex {
    card_index("74716642-fc8c-4f62-a556-154ed0b4af0e")
}
fn orcs_card() -> CardIndex {
    // Ironclaw Orcs, a red creature.
    card_index("c645a616-0d7d-416c-b5f3-057b3a1666a0")
}
fn infantry_card() -> CardIndex {
    // Icatian Infantry, a white creature.
    card_index("9eb274f3-ef55-4545-81b4-29454a48f075")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// P0 attacks p1 with `attacker`; p1 holds Heroism and `p1_creature`; p0 has
/// three Mountains. Stops with p1 holding priority in the declare attackers
/// step. Returns the engine, the attacker and p1's creature.
fn attack_into_heroism(
    attacker: CardIndex,
    p1_creature: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[attacker, mountain(), mountain(), mountain()])
        .battlefield(1, &[heroism_card(), p1_creature])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let attacker = on_battlefield(&e, P0, attacker).expect("the attacker");
    let defender = on_battlefield(&e, P1, p1_creature).expect("p1's creature");

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(P1))],
        },
    )
    .expect("p0 attacks");
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    (e, attacker, defender)
}

/// Presses Heroism, sacrificing `victim`.
#[track_caller]
fn press_heroism(e: &mut Engine<RegistryLookup>, victim: ObjectId) {
    let heroism = on_battlefield(e, P1, heroism_card()).expect("Heroism");
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: heroism,
            ability_index: 0,
        },
    )
    .expect("Heroism's ability is offered");
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("the sacrifice is a choice, got {:?}", e.pending())
    };
    assert!(options.contains(&victim), "a white creature is on the menu");
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![victim],
        },
    )
    .expect("the victim was on the menu");
}

fn the_question(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayMana { .. },
            ..
        }
    )
}

/// Heroism (74716642-fc8c-4f62-a556-154ed0b4af0e) — "Sacrifice a white
/// creature: For each attacking red creature, prevent all combat damage that
/// would be dealt by that creature this turn unless its controller pays
/// {2}{R}." The attacker's controller declines, and nothing is dealt.
#[test]
fn heroism_prevents_the_red_attackers_damage_when_its_controller_does_not_pay() {
    let (mut e, orcs, infantry) = attack_into_heroism(orcs_card(), infantry_card());
    assert_eq!(e.state().players[1].life, 20);
    press_heroism(&mut e, infantry);
    assert_eq!(
        e.state().object(infantry).map(|o| o.zone),
        Some(Zone::Graveyard),
        "sacrificed as the cost"
    );

    pass_until(&mut e, the_question);
    let Pending::YesNo { player, prompt, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the red creature's controller decides");
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{2}{R}")
        }
    );
    e.apply(P0, PlayerAction::YesNo(false)).expect("decline");
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        e.state().players[1].life,
        20,
        "the Orcs' combat damage was prevented"
    );
    assert_eq!(
        e.state().object(orcs).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
}

/// Heroism — "unless its controller pays {2}{R}": paying lets the damage
/// through.
#[test]
fn heroism_lets_the_damage_through_when_the_attackers_controller_pays() {
    let (mut e, orcs, infantry) = attack_into_heroism(orcs_card(), infantry_card());
    let (power, _) = pt(&e, orcs);
    press_heroism(&mut e, infantry);

    pass_until(&mut e, the_question);
    e.apply(P0, PlayerAction::YesNo(true)).expect("pay");
    tap_all_mana(&mut e, P0);
    e.apply(P0, PlayerAction::PassPriority)
        .expect("closing the payment window");
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        e.state().players[1].life,
        20 - i32::from(power),
        "paid: the Orcs hit for their full power"
    );
}

/// Heroism — "each attacking red creature": a green attacker is not asked
/// about and still deals its damage, though the sacrifice was paid.
#[test]
fn heroism_ignores_attackers_that_are_not_red() {
    let (mut e, bears, infantry) = attack_into_heroism(bears_card(), infantry_card());
    let (power, _) = pt(&e, bears);
    press_heroism(&mut e, infantry);
    pass_until(&mut e, |e| the_question(e) || stack_is_empty(e));
    assert!(!the_question(&e), "nobody is asked about a green creature");
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().players[1].life, 20 - i32::from(power));
}

/// Heroism — "Sacrifice a white creature": a creature of another colour
/// cannot pay.
#[test]
fn heroism_cannot_be_paid_with_a_nonwhite_creature() {
    let (e, _orcs, _bears) = attack_into_heroism(orcs_card(), bears_card());
    let heroism = on_battlefield(&e, P1, heroism_card()).expect("Heroism");
    assert!(
        !priority_offer(&e).abilities.contains(&(heroism, 0)),
        "a green creature is not a white one"
    );
}
