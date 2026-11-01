//! Baron Sengir — `{5}{B}{B}{B}` 5/5 Legendary Creature — Vampire Noble (Homelands)
//!
//! Flying
//! Whenever a creature dealt damage by Baron Sengir this turn dies, put a
//! +2/+2 counter on Baron Sengir.
//! `{T}`: Regenerate another target Vampire.

#[allow(clippy::wildcard_imports)]
use super::*;

fn baron_sengir() -> CardIndex {
    card_index("fc728cd7-2b5b-45f3-896a-a45648bcb61d")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

fn board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[lightning_bolt(), lightning_bolt(), lightning_bolt()])
        .battlefield(
            0,
            &[
                baron_sengir(),
                sengir_vampire(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[sengir_vampire(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine
}

fn kill_blocking_vampire(
    engine: &mut Engine<RegistryLookup>,
    baron: ObjectId,
    opp_vampire: ObjectId,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(baron, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(opp_vampire, baron)],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

fn bolt_target(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: ObjectId) {
    let bolt = in_hand(engine, player, lightning_bolt()).unwrap();
    engine
        .apply(player, PlayerAction::CastSpell { card: bolt })
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
fn baron_sengir_gains_plus_two_plus_two_counter_on_kill_and_regenerates_another_vampire() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();

    let baron = on_battlefield(&engine, p0, baron_sengir()).unwrap();
    let ally_vampire = on_battlefield(&engine, p0, sengir_vampire()).unwrap();
    let opp_vampire = on_battlefield(&engine, p1, sengir_vampire()).unwrap();
    let opp_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, baron), (5, 5));
    assert!(keywords(&engine, baron).contains(KeywordSet::FLYING));

    // Attack with Baron Sengir; p1 blocks with 4/4 Sengir Vampire.
    kill_blocking_vampire(&mut engine, baron, opp_vampire);

    // Opponent's 4/4 Sengir Vampire died after being dealt 5 damage by Baron Sengir -> +2/+2 counter!
    assert_eq!(
        engine.state().object(opp_vampire).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine
            .state()
            .object(baron)
            .unwrap()
            .counters
            .get(CounterKind::Plus {
                power: 2,
                toughness: 2,
            }),
        1
    );
    assert_eq!(pt(&engine, baron), (7, 7));

    // On p0's next turn, Baron Sengir untaps and uses {T}: Regenerate another target Vampire.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: baron,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        !options.contains(&baron),
        "Baron Sengir can only target *another* Vampire, not itself"
    );
    assert!(options.contains(&ally_vampire));
    assert!(
        !options.contains(&opp_bears),
        "Grizzly Bears is not a Vampire"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ally_vampire],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast two Lightning Bolts (6 damage) at 4/4 ally_vampire: it regenerates!
    tap_all_mana(&mut engine, p0);
    for _ in 0..2 {
        bolt_target(&mut engine, p0, ally_vampire);
    }
    assert_eq!(
        engine.state().object(ally_vampire).unwrap().zone,
        Zone::Battlefield,
        "another target Vampire regenerated from lethal damage via Baron Sengir"
    );
    assert!(is_tapped(&engine, ally_vampire));

    // A creature that Baron Sengir did not damage dies: no counter. Lightning
    // Bolt kills the opponent's Grizzly Bears; Baron Sengir is still 7/7.
    bolt_target(&mut engine, p0, opp_bears);
    assert_eq!(
        engine.state().object(opp_bears).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine
            .state()
            .object(baron)
            .unwrap()
            .counters
            .get(CounterKind::Plus {
                power: 2,
                toughness: 2,
            }),
        1,
        "a creature not dealt damage by Baron Sengir dying adds no counter"
    );
    assert_eq!(pt(&engine, baron), (7, 7));
}

/// The subtype restriction admits Kindred permanents too, and does not
/// require the Vampire to be controlled by Baron's controller.
#[test]
fn baron_sengir_regenerates_a_noncreature_kindred_vampire_but_not_itself() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::{
        generated::{index, subtypes},
        types::SubtypeSet,
    };
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[baron_sengir(), forest()],
        &[sol_ring()],
        &[index::DISENCHANT],
        &[],
    );
    let baron = lk::obj(&e, p0, baron_sengir());
    let kindred = lk::obj(&e, p1, sol_ring());
    let land = lk::obj(&e, p0, forest());
    // There is no real Kindred fixture in this pool. Keep a real artifact
    // and give this controlled fixture the Vampire subtype and Kindred type.
    let state = e.dev_state_mut(p0).unwrap();
    let base = std::sync::Arc::make_mut(&mut state.object_mut(kindred).unwrap().base);
    base.types = TypeSet::KINDRED.union(TypeSet::ARTIFACT);
    base.subtypes = SubtypeSet::from_slice(&[subtypes::creature::VAMPIRE]);
    state.invalidate_projections();
    state.refresh_characteristics();
    e.refresh_offer();
    activate(&mut e, p0, baron_sengir(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("choose another Vampire")
    };
    assert!(options.contains(&kindred));
    assert!(!options.contains(&baron) && !options.contains(&land));
    lk::aim(&mut e, &[kindred], &[]);
    lk::settle(&mut e);
    lk::float(&mut e, p0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut e, p0, index::DISENCHANT);
    lk::aim(&mut e, &[kindred], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(e.pending(), Pending::YesNo { .. }) {
        lk::yes_no(&mut e, true);
    }
    lk::settle(&mut e);
    assert_eq!(e.state().object(kindred).unwrap().zone, Zone::Battlefield);
    assert!(
        is_tapped(&e, kindred),
        "the noncreature Vampire regenerated"
    );
    assert_eq!(e.state().object(kindred).unwrap().regeneration_shields, 0);
}
