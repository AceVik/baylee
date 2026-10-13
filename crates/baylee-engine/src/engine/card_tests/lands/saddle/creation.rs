//! Each Road creates the same printed Pilot, at sorcery speed and with real costs.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn every_road_sacrifices_for_two_pilots_under_doubling_season() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    for (land, color) in [
        (index::COUNTRY_ROADS, ManaColor::White),
        (index::REEF_ROADS, ManaColor::Blue),
        (index::FOUL_ROADS, ManaColor::Black),
        (index::ROCKY_ROADS, ManaColor::Red),
        (index::WILD_ROADS, ManaColor::Green),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[land, index::DOUBLING_SEASON])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let source = on_battlefield(&e, p, land).unwrap();
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(color, 2);
        e.refresh_offer();
        e.apply(p, PlayerAction::PassPriority).unwrap();
        e.apply(q, PlayerAction::PassPriority).unwrap();
        // This advances out of main; the activation is no longer a sorcery action.
        assert!(
            e.apply(
                p,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 1
                }
            )
            .is_err()
        );
        reach_their_main_phase(&mut e, q);
        reach_their_main_phase(&mut e, p);
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(color, 2);
        e.refresh_offer();
        e.apply(
            p,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 1,
            },
        )
        .unwrap();
        assert_eq!(e.state().object(source).unwrap().zone, Zone::Graveyard);
        assert!(tokens_of(&e, p).is_empty());
        pass_until(&mut e, stack_is_empty);
        let made = tokens_of(&e, p);
        assert_eq!(made.len(), 2);
        for id in made {
            assert_eq!(pt(&e, id), (1, 1));
            assert!(std::ptr::eq(
                e.state().object(id).unwrap().token.unwrap(),
                &raw const baylee_cards::tokens::PILOT_1_1_COLORLESS
            ));
        }
    }
}

/// "Activate only as a sorcery." The Roads' other price is `{1}` of their colour,
/// so every refusal below is made with that mana floating and the Road
/// untapped: the only thing left to refuse the activation is the timing. The
/// stack case also ends with the very same activation accepted once the stack
/// is empty, so the refusal cannot have been about anything else.
#[test]
fn every_road_sacrifices_only_as_a_sorcery_with_the_mana_in_hand() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    for (land, color) in [
        (index::COUNTRY_ROADS, ManaColor::White),
        (index::REEF_ROADS, ManaColor::Blue),
        (index::FOUL_ROADS, ManaColor::Black),
        (index::ROCKY_ROADS, ManaColor::Red),
        (index::WILD_ROADS, ManaColor::Green),
    ] {
        let name = baylee_cards::by_index(land).map_or("?", baylee_cards_dsl::CardDef::name);
        let offered = |e: &Engine<RegistryLookup>, source: ObjectId| {
            let Pending::Priority { player, legal, .. } = e.pending() else {
                panic!("{name}: expected priority, got {:?}", e.pending())
            };
            assert_eq!(*player, p, "{name}: the activating seat holds priority");
            legal.abilities.contains(&(source, 1))
        };
        let sacrifice = |e: &mut Engine<RegistryLookup>, source: ObjectId| {
            e.apply(
                p,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 1,
                },
            )
        };

        // Not a step of the main phase: the mana is re-added after the step
        // change emptied the pool, so it is floating when the refusal comes.
        let mut e = Duel::new(SEED, forest()).battlefield(0, &[land]).start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let source = on_battlefield(&e, p, land).unwrap();
        e.apply(p, PlayerAction::PassPriority).unwrap();
        e.apply(q, PlayerAction::PassPriority).unwrap();
        assert!(
            !matches!(e.state().turn.phase, Phase::FirstMain | Phase::SecondMain),
            "{name}: the pass left the main phase"
        );
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(color, 2);
        e.refresh_offer();
        assert!(!is_tapped(&e, source), "{name}: the Road is untapped");
        assert!(
            !offered(&e, source),
            "{name}: outside a main phase the sacrifice is not offered"
        );
        assert!(
            sacrifice(&mut e, source).is_err(),
            "{name}: only as a sorcery: refused outside a main phase with its mana floating"
        );
        assert_eq!(e.state().object(source).unwrap().zone, Zone::Battlefield);

        // A main phase of its own with something on the stack.
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[land])
            .hand(0, &[llanowar_elves()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let source = on_battlefield(&e, p, land).unwrap();
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Green, 1);
        e.refresh_offer();
        cast_with_floating(&mut e, p, llanowar_elves());
        assert!(!stack_is_empty(&e), "{name}: the Elves wait on the stack");
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(color, 2);
        e.refresh_offer();
        assert!(
            !offered(&e, source),
            "{name}: with a spell on the stack the sacrifice is not offered"
        );
        assert!(
            sacrifice(&mut e, source).is_err(),
            "{name}: only as a sorcery: refused over a non-empty stack in its own main phase"
        );
        assert_eq!(e.state().object(source).unwrap().zone, Zone::Battlefield);
        pass_until(&mut e, stack_is_empty);
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(color, 2);
        e.refresh_offer();
        assert!(
            offered(&e, source),
            "{name}: the same Road, the same mana, an empty stack: offered"
        );
        sacrifice(&mut e, source).expect("accepted once the stack is empty");
        assert_eq!(e.state().object(source).unwrap().zone, Zone::Graveyard);
    }
}
