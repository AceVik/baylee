//! `cards/enchantments/auras/mv_4/merseine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn merseine_card() -> CardIndex {
    card_index("b6b6e085-eef8-486d-9d66-1b7f1ee7d0dc")
}

fn fill_pool(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

/// P0 holds Merseine in hand; P1 has a tapped Grizzly Bears and two Forests.
/// Merseine is cast on the Bears and has resolved. Returns the engine, the
/// Bears and the Aura.
fn merseine_on_their_tapped_bears() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let bears_card = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0");
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[merseine_card()])
        .battlefield(1, &[bears_card, forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bears = on_battlefield(&e, P1, bears_card).expect("P1's Grizzly Bears");
    e.state.set_tapped(bears, true);

    fill_pool(&mut e);
    let aura = in_hand(&e, P0, merseine_card()).expect("Merseine in hand");
    e.apply(P0, PlayerAction::CastSpell { card: aura })
        .expect("Merseine is castable");
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![bears],
            players: vec![],
        },
    )
    .expect("the Bears are a legal target");
    pass_until(&mut e, stack_is_empty);
    let aura = on_battlefield(&e, P0, merseine_card()).expect("Merseine resolved");
    (e, bears, aura)
}

fn net_counters(e: &Engine<RegistryLookup>, aura: ObjectId) -> u16 {
    e.state()
        .object(aura)
        .expect("the Aura")
        .counters
        .get(baylee_cards_dsl::counters::NET)
}

fn is_tapped(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .expect("object exists")
        .status
        .contains(Status::TAPPED)
}

fn p1_pays_a_counter(e: &mut Engine<RegistryLookup>, aura: ObjectId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    fill_pool(e);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 2,
        },
    )
    .expect("P1 pays to remove a net counter");
    pass_until(e, stack_is_empty);
}

/// Merseine (b6b6e085-eef8-486d-9d66-1b7f1ee7d0dc) — "Enchant creature. This
/// Aura enters with three net counters on it. Enchanted creature doesn't
/// untap during its controller's untap step if this Aura has a net counter on
/// it."
#[test]
fn merseine_enters_with_three_net_counters_and_keeps_the_creature_tapped() {
    let (mut e, bears, aura) = merseine_on_their_tapped_bears();
    assert_eq!(net_counters(&e, aura), 3, "enters with three net counters");
    assert!(is_tapped(&e, bears));

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(
        is_tapped(&e, bears),
        "the enchanted creature did not untap in its controller's untap step"
    );
}

/// Merseine — "Pay enchanted creature's mana cost: Remove a net counter from
/// this Aura. Only the controller of the enchanted creature may activate this
/// ability."
#[test]
fn merseine_only_the_enchanted_creatures_controller_may_pay_to_remove_a_counter() {
    let (mut e, _bears, aura) = merseine_on_their_tapped_bears();

    fill_pool(&mut e);
    let offer = priority_offer(&e);
    assert!(
        !offer.abilities.contains(&(aura, 2)),
        "the Aura's controller may not pay its removal cost"
    );

    // P1 gets priority and is the one who may pay.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    fill_pool(&mut e);
    let offer = priority_offer(&e);
    assert!(
        offer.abilities.contains(&(aura, 2)),
        "the enchanted creature's controller may pay"
    );

    let before = e.state.players[1].mana_pool.total();
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 2,
        },
    )
    .expect("P1 activates Merseine's removal ability");
    assert_eq!(
        before - e.state.players[1].mana_pool.total(),
        2,
        "the cost was the Bears' mana cost, {{1}}{{G}}"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(net_counters(&e, aura), 2);
}

/// Merseine — the creature stays tapped as long as a single net counter
/// remains, and untaps in the untap step once the last one is gone.
#[test]
fn merseine_releases_the_creature_only_after_the_last_net_counter_is_paid_off() {
    let (mut e, bears, aura) = merseine_on_their_tapped_bears();

    p1_pays_a_counter(&mut e, aura);
    p1_pays_a_counter(&mut e, aura);
    assert_eq!(net_counters(&e, aura), 1);

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(is_tapped(&e, bears), "one net counter left: still no untap");

    // P1's own turn: it may pay the last counter at instant speed.
    p1_pays_a_counter(&mut e, aura);
    assert_eq!(net_counters(&e, aura), 0, "the last counter is gone");
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(
        !is_tapped(&e, bears),
        "with no net counter left, the creature untaps"
    );
}
