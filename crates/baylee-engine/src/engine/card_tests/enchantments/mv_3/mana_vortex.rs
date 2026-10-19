//! `cards/enchantments/mv_3/mana_vortex.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn vortex_card() -> CardIndex {
    card_index("98b88734-09e2-4209-a787-9b5b345391f9")
}

fn lands_of(e: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = e.state().object(id).expect("on the battlefield");
            o.controller == seat && o.characteristics().types.contains(TypeSet::LAND)
        })
        .collect()
}

fn asked_to_choose(e: &Engine<RegistryLookup>) -> bool {
    matches!(e.pending(), Pending::ChooseCards { .. })
}

/// P0: three Islands and two Forests, Mana Vortex in hand. P1: two Forests.
/// The Vortex is cast off all of P0's lands, and the game stops on the
/// question its cast trigger asks.
fn vortex_cast() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, swamp())
        .hand(0, &[vortex_card()])
        .battlefield(0, &[island(), island(), island(), forest(), forest()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, vortex_card());
    pass_until(&mut e, asked_to_choose);
    e
}

/// Mana Vortex (98b88734-09e2-4209-a787-9b5b345391f9) — "When you cast this
/// spell, counter it unless you sacrifice a land." Sacrificing one keeps the
/// spell.
#[test]
fn mana_vortex_resolves_when_its_caster_sacrifices_a_land() {
    let mut e = vortex_cast();
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the caster decides");
    let mine = lands_of(&e, P0);
    assert_eq!(mine.len(), 5);
    for l in &mine {
        assert!(options.contains(l), "each of the caster's lands is offered");
    }
    let victim = on_battlefield(&e, P0, forest()).expect("a Forest");
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![victim],
        },
    )
    .expect("a land is a legal sacrifice");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().object(victim).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the land was sacrificed"
    );
    assert!(
        on_battlefield(&e, P0, vortex_card()).is_some(),
        "and the Vortex resolved"
    );
}

/// Mana Vortex — declining the sacrifice counters the spell.
#[test]
fn mana_vortex_is_countered_when_its_caster_sacrifices_nothing() {
    let mut e = vortex_cast();
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("sacrificing none is allowed");
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, vortex_card()).is_none(),
        "countered: not on the battlefield"
    );
    assert!(
        in_graveyard(&e, P0, vortex_card()).is_some(),
        "countered spells go to the graveyard"
    );
    assert_eq!(lands_of(&e, P0).len(), 5, "no land was sacrificed");
}

/// Mana Vortex — "unless you sacrifice a land" with no land to sacrifice: the
/// spell is simply countered, and nothing is asked.
#[test]
fn mana_vortex_with_no_land_to_sacrifice_is_countered() {
    let mut e = Duel::new(SEED, swamp()).hand(0, &[vortex_card()]).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for c in ManaColor::ALL {
        e.state.players[0].mana_pool.add(c, 5);
    }
    e.refresh_offer();
    cast_with_floating(&mut e, P0, vortex_card());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, vortex_card()).is_none());
    assert!(in_graveyard(&e, P0, vortex_card()).is_some());
}

/// Mana Vortex — "At the beginning of each player's upkeep, that player
/// sacrifices a land of their choice." The opponent's upkeep first, then the
/// controller's own.
#[test]
fn mana_vortex_makes_each_player_sacrifice_a_land_in_their_upkeep() {
    let mut e = vortex_cast();
    let victim = on_battlefield(&e, P0, forest()).expect("a Forest");
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![victim],
        },
    )
    .expect("a land is a legal sacrifice");
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, vortex_card()).is_some());

    // p1's upkeep.
    pass_until(&mut e, asked_to_choose);
    assert_eq!(e.state().turn.active, P1);
    assert_eq!(e.state().turn.step, crate::turn::Step::Upkeep);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P1, "that player chooses");
    assert_eq!((min, max), (1, 1), "exactly one land");
    let theirs = lands_of(&e, P1);
    assert_eq!(theirs.len(), 2);
    let mut sorted_options = options.clone();
    sorted_options.sort_unstable();
    let mut sorted_theirs = theirs.clone();
    sorted_theirs.sort_unstable();
    assert_eq!(sorted_options, sorted_theirs, "only their own lands");
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![theirs[0]],
        },
    )
    .expect("one of their lands");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(theirs[0]).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(lands_of(&e, P1).len(), 1);

    // p0's own upkeep.
    pass_until(&mut e, |e| {
        asked_to_choose(e) && e.state().turn.active == P0
    });
    assert_eq!(e.state().turn.step, crate::turn::Step::Upkeep);
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(
        player, P0,
        "the controller is asked in their own upkeep too"
    );
    assert_eq!(options.len(), 4, "p0 has four lands left to choose from");
    let before = lands_of(&e, P0).len();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .expect("one of p0's lands");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lands_of(&e, P0).len(), before - 1);
}

/// Mana Vortex — "When there are no lands on the battlefield, sacrifice this
/// enchantment." With no land in play it goes at once; with one anywhere it
/// stays.
#[test]
fn mana_vortex_is_sacrificed_when_no_lands_are_on_the_battlefield() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[vortex_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, vortex_card()).is_none(),
        "no lands anywhere: sacrificed"
    );
    assert!(in_graveyard(&e, P0, vortex_card()).is_some());

    // Control: an opponent's land anywhere keeps it.
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[vortex_card()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, vortex_card()).is_some(),
        "a land on the battlefield, whoever's it is: the Vortex stays"
    );
}
