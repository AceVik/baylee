//! `cards/lands/artifacts/power_depot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The printed mana or land-face ability is exercised through player actions.
#[test]
fn power_depot_enters_tapped_with_counter_and_adds_restricted_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[power_depot()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, power_depot());
    assert!(entered_tapped(&engine, land));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 1);
    let t = types(&engine, land);
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::LAND));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, power_depot(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert!(is_tapped(&engine, land));
}

#[test]
fn modular_uses_departure_counters_even_if_the_source_returns_and_may_target_an_opponents_creature()
{
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for card in [index::POWER_DEPOT, index::ARCBOUND_RAVAGER] {
        for (decline, recipient) in [(false, p0), (false, p1), (true, p0)] {
            let mut e = Duel::new(SEED, forest())
                .battlefield(0, &[card, index::ORNITHOPTER, index::DOUBLING_SEASON])
                .battlefield(1, &[index::ORNITHOPTER])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let source = on_battlefield(&e, p0, card).unwrap();
            let target = on_battlefield(&e, recipient, index::ORNITHOPTER).unwrap();
            assert_eq!(counters_on(&e, source, CounterKind::P1P1), 2);
            {
                let s = e.dev_state_mut(p0).unwrap();
                crate::replacement::put_counters(s, source, CounterKind::P1P1, 2);
            }
            assert_eq!(counters_on(&e, source, CounterKind::P1P1), 6);
            bury(&mut e, &[source]);
            e.apply(p0, PlayerAction::PassPriority).unwrap();
            let Pending::ChooseTargets { player, .. } = e.pending().clone() else {
                panic!("modular target: {:?}", e.pending())
            };
            aim_at(&mut e, player, target);
            {
                let s = e.dev_state_mut(p0).unwrap();
                s.object_mut(source).unwrap().kind = crate::object::ObjectKind::Permanent;
                s.move_object(
                    source,
                    ZoneLocation::Battlefield,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
            }
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: crate::choice::YesNoPrompt::MayDo,
                        ..
                    }
                )
            });
            e.apply(p0, PlayerAction::YesNo(!decline)).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                counters_on(&e, target, CounterKind::P1P1),
                if decline {
                    0
                } else if recipient == p0 {
                    12
                } else {
                    6
                }
            );
            assert_eq!(
                counters_on(&e, source, CounterKind::P1P1),
                2,
                "new permanent's counters are separate"
            );
        }
    }
}

fn mind_stone() -> CardIndex {
    baylee_core::generated::index::MIND_STONE
}

/// A table where only the Depot's restricted mana floats: the Depot has been
/// tapped for one mana of green, and nothing else makes any. `board` is what
/// stands beside the Depot: a Mind Stone is one more mana ability, so a spell
/// is then offered to be paid for while it is cast (`LegalActions::payable`),
/// which is not what a test about the mana floating already wants to read.
fn only_the_depots_restricted_mana(
    board: &[CardIndex],
    hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[&[power_depot()], board].concat())
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, power_depot(), 1);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana, and it is the Depot's");
    assert_eq!(pool.restricted().len(), 1, "and it is restricted");
    engine
}

/// "Spend this mana only to cast artifact spells or activate abilities of
/// artifacts": it pays {1} of Mind Stone's "{1}, {T}, Sacrifice" ability.
#[test]
fn power_depot_restricted_mana_pays_the_activation_of_an_artifacts_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = only_the_depots_restricted_mana(&[mind_stone()], &[]);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, mind_stone(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, mind_stone()).is_none(),
        "the Mind Stone was sacrificed as part of the cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and its ability drew a card"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Depot's restricted mana paid the {{1}} and nothing is left"
    );
}

/// The same mana cannot pay for a spell that is not an artifact, though an
/// artifact spell of the same price takes it.
#[test]
fn power_depot_restricted_mana_cannot_cast_a_creature_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = only_the_depots_restricted_mana(&[], &[llanowar_elves(), sol_ring()]);
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("Llanowar Elves in hand");
    let ring = in_hand(&engine, p0, sol_ring()).expect("Sol Ring in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ring),
        "an artifact spell can be paid with the Depot's mana: {legal:?}"
    );
    assert!(
        !legal.castable.contains(&elves) && !legal.payable.contains(&elves),
        "a creature spell that is no artifact cannot, and no other source could pay for it either: {legal:?}"
    );

    let before = engine.snapshot_hash();
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: elves })
            .is_err(),
        "casting the Elves with only the restricted {{G}} is refused"
    );
    assert_eq!(
        engine.snapshot_hash(),
        before,
        "and the refusal changed nothing"
    );
}

/// Modular 1: "target artifact creature" — a creature that is no artifact and
/// an artifact that is no creature are not on the menu.
#[test]
fn modular_offers_only_artifact_creatures() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                power_depot(),
                index::ORNITHOPTER,
                index::GRIZZLY_BEARS,
                index::SOL_RING,
            ],
        )
        .battlefield(1, &[index::ORNITHOPTER, llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let depot = on_battlefield(&e, p0, power_depot()).unwrap();
    let mine = on_battlefield(&e, p0, index::ORNITHOPTER).unwrap();
    let theirs = on_battlefield(&e, p1, index::ORNITHOPTER).unwrap();

    bury(&mut e, &[depot]);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseTargets { mut options, .. } = e.pending().clone() else {
        panic!("modular target: {:?}", e.pending())
    };

    options.sort();
    let mut expected = vec![mine, theirs];
    expected.sort();
    assert_eq!(
        options, expected,
        "only the two Ornithopters (artifact creatures) are targets; not the \
         Bears or the Elves (creatures), nor the Sol Ring (an artifact)"
    );
}
