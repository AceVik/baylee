//! `cards/enchantments/auras/mv_1/thrull_retainer.rs` and Briar Shield,
//! played: an Aura sacrificed to pay its own ability still affects the
//! creature it enchanted as it left (CR 113.7a, CR 608.2h).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::aura_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const BLACK: (ManaColor, u32) = (ManaColor::Black, 1);
const GREEN: (ManaColor, u32) = (ManaColor::Green, 1);

fn thrull_retainer() -> CardIndex {
    card_index("81a47019-1343-4a63-893a-0d8ba5e848d5")
}

fn briar_shield() -> CardIndex {
    card_index("9d437391-d476-4976-8714-6a1d3185ab52")
}

/// Rend Flesh at the enchanted Bears, Thrull Retainer sacrificed in
/// response: the shield goes on the Bears, and Rend Flesh only taps them
/// (Argon v Astra, turn 54: Knight of Stromgald died instead).
#[test]
fn a_sacrificed_retainer_regenerates_the_creature_it_enchanted() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[grizzly_bears()])
        .hand(0, &[thrull_retainer()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[rend_flesh()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bears = on_battlefield(&e, P0, grizzly_bears()).unwrap();
    kit::cast_aura(&mut e, P0, thrull_retainer(), bears, &[BLACK]);

    e.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut e, P1);
    let flesh = in_hand(&e, P1, rend_flesh()).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: flesh })
        .unwrap();
    e.apply(
        P1,
        PlayerAction::ChooseTargets {
            objects: vec![bears],
            players: vec![],
        },
    )
    .unwrap();
    e.apply(P1, PlayerAction::PassPriority).unwrap();
    activate(&mut e, P0, thrull_retainer(), 2);
    assert!(
        in_graveyard(&e, P0, thrull_retainer()).is_some(),
        "the Aura paid for its ability"
    );
    pass_until(&mut e, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    assert_eq!(
        e.state().object(bears).unwrap().regeneration_shields,
        1,
        "the creature it enchanted as it left is shielded"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "regenerated, not destroyed"
    );
    assert!(is_tapped(&e, bears), "regeneration taps it");
}

/// The same reading for a pump: Briar Shield sacrificed gives the creature
/// it enchanted +3/+3, its own +1/+1 gone with it.
#[test]
fn a_sacrificed_briar_shield_pumps_the_creature_it_enchanted() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .hand(0, &[briar_shield()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bears = on_battlefield(&e, P0, grizzly_bears()).unwrap();
    kit::cast_aura(&mut e, P0, briar_shield(), bears, &[GREEN]);
    assert_eq!(pt(&e, bears), (3, 3));
    activate(&mut e, P0, briar_shield(), 2);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, briar_shield()).is_some());
    assert_eq!(pt(&e, bears), (5, 5), "2/2 and +3/+3");
}
