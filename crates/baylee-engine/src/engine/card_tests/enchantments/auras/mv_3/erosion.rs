//! `cards/enchantments/auras/mv_3/erosion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// A duel at P0's first main phase with the named hand and boards, and every
/// colour of mana floating for both seats.
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).expect("the spell is in hand");
    e.apply(p, PlayerAction::CastSpell { card })
        .expect("the spell is cast");
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects,
            players: vec![],
        },
    )
    .expect("the targets are on the menu");
}

fn yes_no(e: &mut Engine<RegistryLookup>, answer: bool) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::YesNo(answer))
        .expect("the question takes a yes or a no");
}

fn erosion() -> CardIndex {
    card_index("155ee24f-a458-4df4-86d5-5eb7396ebf45")
}

/// Erosion on P1's Forest, waiting at P1's first upkeep for the question.
fn erosion_at_the_upkeep_question() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = prepared(&[erosion()], &[], &[forest()]);
    let land = on_battlefield(&e, P1, forest()).expect("P1's Forest is out");
    cast(&mut e, P0, erosion());
    targets(&mut e, vec![land]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, erosion()).is_some(),
        "the Aura is out"
    );

    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, prompt, .. } = e.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, P1, "the enchanted land's controller is asked");
    assert!(
        matches!(prompt, YesNoPrompt::PayTax { mana: 1 }),
        "{{1}} first, got {prompt:?}"
    );
    assert_eq!(e.state().turn.number, 2);
    assert_eq!(e.state().turn.step, Step::Upkeep);
    (e, land)
}

/// Erosion (155ee24f-a458-4df4-86d5-5eb7396ebf45) — "Enchant land. At the
/// beginning of the upkeep of enchanted land's controller, destroy that land
/// unless that player pays {1} or 1 life." (pays {1})
#[test]
fn erosion_spares_the_land_when_its_controller_pays_one_mana() {
    let (mut e, land) = erosion_at_the_upkeep_question();
    e.state.players[1].mana_pool.add(ManaColor::Green, 1);
    yes_no(&mut e, true);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().players[1].life, 20, "no life paid");
    assert_eq!(
        e.state().players[1].mana_pool.total(),
        0,
        "the {{1}} was spent"
    );

    // The Aura names the *enchanted land's* controller, not its own: P0's
    // upkeep asks nothing (pass_until would refuse a question it cannot answer).
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep
    });
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
}

/// Erosion — (pays 1 life)
#[test]
fn erosion_spares_the_land_when_its_controller_pays_one_life() {
    let (mut e, land) = erosion_at_the_upkeep_question();
    yes_no(&mut e, false);
    assert!(
        matches!(
            e.pending(),
            Pending::YesNo {
                player: P1,
                prompt: YesNoPrompt::PayLife { .. },
                ..
            }
        ),
        "declining the mana, the life is asked: {:?}",
        e.pending()
    );
    yes_no(&mut e, true);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().players[1].life, 19, "one life paid");
}

/// Erosion — (pays neither)
#[test]
fn erosion_destroys_the_land_when_its_controller_pays_nothing() {
    let (mut e, land) = erosion_at_the_upkeep_question();
    yes_no(&mut e, false);
    yes_no(&mut e, false);
    pass_until(&mut e, stack_is_empty);

    assert_ne!(
        e.state().object(land).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the land is destroyed"
    );
    assert!(in_graveyard(&e, P1, forest()).is_some());
    assert_eq!(e.state().players[1].life, 20, "and no life was paid");
    assert!(
        on_battlefield(&e, P0, erosion()).is_none(),
        "the Aura falls off with it"
    );
}
