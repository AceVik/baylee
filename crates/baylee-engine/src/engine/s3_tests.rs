use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn polluted_delta() -> CardIndex {
    card_index("ef86989d-ce80-4e55-aece-7d11710eeffa")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn harabaz_druid() -> CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}
/// A Plains Island that enters tapped — an Island a fetchland may find, and
/// one that carries its own entry replacement.
fn irrigated_farmland() -> CardIndex {
    card_index("406eabe2-df62-49e2-bb39-c0227509d875")
}
/// A shockland: the same fetch target, but its replacement *asks* rather
/// than writing a status.
fn hallowed_fountain() -> CardIndex {
    card_index("f1750962-a87c-49f6-b731-02ae971ac6ea")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

/// Preset: each seat gets a fixed starting hand; the library is a
/// forest/island mix.
fn preset_with_hand(seed: u64, hand0: Vec<CardIndex>, hand1: Vec<CardIndex>) -> GamePreset {
    preset_with_library(seed, hand0, hand1, |i| {
        if i % 2 == 0 { island() } else { forest() }
    })
}

/// The same, with the sixty-card library written by `deck` — for a test that
/// needs a particular card to be *findable*.
fn preset_with_library(
    seed: u64,
    hand0: Vec<CardIndex>,
    hand1: Vec<CardIndex>,
    deck: impl Fn(usize) -> CardIndex,
) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|i| entry(deck(i))).collect();
    let mk_seat = |hand: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk_seat(hand0), mk_seat(hand1)],
    }
}

fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

fn pass_to_main(engine: &mut Engine<RegistryLookup>, want_active: PlayerId) {
    let mut guard = 0;
    while !(matches!(engine.state().turn.phase, Phase::FirstMain)
        && engine.state().turn.active == want_active)
    {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while passing to main: {other:?}"),
        }
        guard += 1;
        assert!(guard < 20, "never reached main phase");
    }
}

/// A fetchland pays life and puts its land in **untapped**.
///
/// That is the whole reason it costs a life and Evolving Wilds does not, and
/// this test asserted the opposite for as long as it existed — under the name
/// `fetchland_searches_island_or_swamp_tapped`, holding four of the ten
/// fetchlands in the pool at `Find::BATTLEFIELD_TAPPED` while the other six
/// were right. Every one of those four carried a header quoting the printed
/// "put it onto the battlefield" and a comment saying `Find::BATTLEFIELD`
/// says so; only the code disagreed, and a test written from the code rather
/// than from the card made the disagreement permanent.
#[test]
fn a_fetchland_puts_its_land_in_untapped() {
    let mut engine = Engine::new(
        &preset_with_hand(9, vec![polluted_delta()], vec![forest()]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    pass_to_main(&mut engine, p0);
    // Play the fetchland.
    let delta = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.lands.contains(&delta));
    engine
        .apply(p0, PlayerAction::PlayLand { card: delta })
        .unwrap();

    // Activate the fetch ability.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(legal.abilities.contains(&(delta, 0)));
    let life_before = engine.state().players[0].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: delta,
                ability_index: 0,
            },
        )
        .unwrap();
    // Cost paid: 1 life, land sacrificed to graveyard, ability on stack.
    assert_eq!(engine.state().players[0].life, life_before - 1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&delta)
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);

    // Pass priority twice → ability resolves → search choice.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority for opponent, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected search choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    // Every option is an Island or Swamp (library is island/forest).
    assert!(!options.is_empty());
    for opt in &options {
        let obj = engine.state().object(*opt).unwrap();
        assert!(
            obj.characteristics()
                .subtypes
                .contains(baylee_core::generated::subtypes::land::ISLAND)
        );
    }
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    // Chosen island is on the battlefield, untapped; library was shuffled.
    let obj = engine.state().object(chosen).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(
        !obj.status.contains(crate::object::Status::TAPPED),
        "a basic Island has nothing that would tap it on the way in"
    );
    assert!(
        engine
            .journal()
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::Shuffled { player, .. } if player == p0))
    );
}

/// And a found card keeps its *own* entry replacement.
///
/// The search puts the card onto the battlefield itself rather than through
/// the land-play path, so whether a fetched tapland still enters tapped is a
/// real question about where `apply_enter_modifiers` looks. It reads the
/// journal for zone changes rather than the way a permanent arrived, which
/// this test says out loud instead of leaving it to a reading of the code —
/// and it is the other half of the report that found the four cards above:
/// a fetchland that put every land in tapped and a fetchland that put none in
/// tapped are both wrong, and only the card being found can say which.
#[test]
fn a_fetched_tapland_still_enters_tapped() {
    // A library of nothing but Irrigated Farmland: an Island the Delta may
    // find, and one whose printed text taps it as it enters.
    let mut engine = Engine::new(
        &preset_with_library(11, vec![polluted_delta()], vec![forest()], |_| {
            irrigated_farmland()
        }),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    pass_to_main(&mut engine, p0);

    let delta = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == polluted_delta())
        })
        .expect("the fetchland is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: delta })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: delta,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority for opponent, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search choice, got {:?}", engine.pending());
    };
    let farmland = *options.first().expect("the library is all Farmland");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![farmland],
            },
        )
        .unwrap();

    let obj = engine.state().object(farmland).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(
        obj.status.contains(crate::object::Status::TAPPED),
        "the land brought its own \"enters tapped\" with it"
    );
}

/// The third case, and the one with a real seam in it: a fetched *shockland*.
///
/// `EnterModifier::Tapped` writes a status and returns; `TappedOrPayLife` is
/// the one arm that publishes a `Pending` and returns *mid-scan*, so it has to
/// survive being raised while a search is finishing resolving — `apply` sets
/// the priority pending first and `apply_enter_modifiers` overrides it. If
/// that override lost, a Hallowed Fountain fetched with a Delta would slide
/// onto the battlefield untapped with no question asked, which is exactly the
/// shape of "das gefatchte Land kommt nicht getappt rein". Both halves of the
/// answer are driven here, because a question that is asked and then ignored
/// is the same bug one step later.
#[test]
fn a_fetched_shockland_still_asks_the_question() {
    let mut engine = Engine::new(
        &preset_with_library(13, vec![polluted_delta()], vec![forest()], |_| {
            hallowed_fountain()
        }),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    pass_to_main(&mut engine, p0);

    let delta = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == polluted_delta())
        })
        .expect("the fetchland is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: delta })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: delta,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority for opponent, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search choice, got {:?}", engine.pending());
    };
    let fountain = *options.first().expect("the library is all Fountain");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fountain],
            },
        )
        .unwrap();

    // The question survives the search's own resolution.
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the shockland never asked; pending is {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount: 2 }
    );

    // Declining taps it, which is the half the player reported missing.
    let life = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    let obj = engine.state().object(fountain).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(
        obj.status.contains(crate::object::Status::TAPPED),
        "declining to pay is what taps a shockland"
    );
    assert_eq!(
        engine.state().players[0].life,
        life,
        "and it costs no life to decline"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // one drive loop over the whole choice taxonomy
fn cleric_rally_gains_life_on_ally_etbs() {
    let mut engine = Engine::new(
        &preset_with_hand(
            4,
            vec![plains(), plains(), forest(), ondu_cleric(), harabaz_druid()],
            vec![forest()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_start = engine.state().players[0].life;
    let mut cleric_cast = false;
    let mut druid_cast = false;
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                if player == p0 {
                    if let Some(&land) = legal.lands.first() {
                        engine
                            .apply(player, PlayerAction::PlayLand { card: land })
                            .unwrap();
                    } else if !legal.mana_abilities.is_empty()
                        // Only where the mana can be spent. A pool empties as
                        // the step ends (CR 500.4), so a land tapped in the
                        // upkeep is a land wasted and this loop never casts.
                        && matches!(
                            engine.state().turn.phase,
                            Phase::FirstMain | Phase::SecondMain
                        )
                    {
                        let sources = legal.mana_abilities.clone();
                        for source in sources {
                            engine
                                .apply(player, PlayerAction::ActivateManaAbility { source })
                                .unwrap();
                        }
                    } else if let Some(&card) = legal.castable.iter().find(|c| {
                        let idx = engine.state().object(**c).unwrap().card.unwrap().index;
                        (!cleric_cast && idx == ondu_cleric())
                            || (cleric_cast && !druid_cast && idx == harabaz_druid())
                    }) {
                        let idx = engine.state().object(card).unwrap().card.unwrap().index;
                        engine
                            .apply(player, PlayerAction::CastSpell { card })
                            .unwrap();
                        if idx == ondu_cleric() {
                            cleric_cast = true;
                        } else {
                            druid_cast = true;
                        }
                    } else {
                        engine.apply(player, PlayerAction::PassPriority).unwrap();
                    }
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..count as usize].to_vec(),
                        },
                    )
                    .unwrap();
            }
            // The rally trigger prints "you may", so it asks. This test is
            // about how often it fires and for how much, which is the same
            // arithmetic once the offer is taken; the decision itself has
            // its own pair of tests in `card_tests`.
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::GameOver(_) => panic!("game should not end"),
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(
            guard < 400,
            "no progress; cleric={cleric_cast} druid={druid_cast}"
        );
        // Real Ondu Cleric text: gain life equal to the number of Allies
        // you control — cleric ETB: 1 Ally (+1), druid ETB: 2 Allies (+2).
        if druid_cast && engine.state().players[0].life == life_start + 3 {
            break;
        }
    }
    assert!(cleric_cast && druid_cast);
    assert_eq!(engine.state().players[0].life, life_start + 3);
}
