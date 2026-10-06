use super::*;

/// A land under a Chromatic Lantern taps for any colour, and there is no
/// card anywhere a client could read that off — the ability exists only
/// in the effect table. It is projected for the same reason as an
/// animated land's types: without it a client's mana planner counts that
/// land for nothing and the player taps it by hand.
///
/// The opponent's land in the same test is the half that matters as much:
/// the grant says "lands *you* control", and a projection that ignored
/// the filter would offer the planner a land the engine refuses.
#[test]
fn a_land_under_a_lantern_says_what_it_now_makes() {
    use baylee_engine::choice::{Pending, PlayerAction};

    let lantern = by_oracle_id("539f5396-d99a-417d-a84c-dff7930b5900")
        .expect("Chromatic Lantern is in the pool")
        .index;
    let mut preset = mixed_print_preset();
    let land = DeckEntry {
        card: island(),
        print: PrintRef::new(0),
    };
    preset.seats[0].starting_battlefield = vec![
        land,
        DeckEntry {
            card: lantern,
            print: PrintRef::new(0),
        },
    ];
    preset.seats[1].starting_battlefield = vec![land];

    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let view = player_view(
        engine.state(),
        PlayerId::new(0),
        1,
        None,
        &SeatContext::default(),
        &[],
    );

    let land_of = |seat: u8| {
        view.battlefield
            .iter()
            .find(|o| {
                o.controller == PlayerId::new(seat)
                    && o.types.contains(baylee_core::types::TypeSet::LAND)
            })
            .expect("each seat has its land")
    };
    let granted = land_of(0)
        .granted_mana
        .as_ref()
        .expect("the Lantern grants the land an ability");
    assert_eq!(granted.amount, 1, "one mana, of a colour it will ask for");
    assert_eq!(
        granted.colors.len(),
        5,
        "any colour, and the client has to know which five"
    );

    assert!(
        land_of(1).granted_mana.is_none(),
        "the grant is `lands you control` and the opponent is not you"
    );
    let lantern_itself = view
        .battlefield
        .iter()
        .find(|o| o.types.contains(baylee_core::types::TypeSet::ARTIFACT))
        .expect("the Lantern is on the battlefield");
    assert!(
        lantern_itself.granted_mana.is_none(),
        "the Lantern's own mana ability is printed on it and is not a grant"
    );

    // And the half that makes the projection worth anything: the engine
    // offers this exact land under this exact handle. A view that said a
    // land makes mana the engine will not hand out is worse than one that
    // said nothing — the planner would tap it and the payment would fail.
    let land = land_of(0).id;
    for _ in 0..30 {
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            break;
        };
        if player == PlayerId::new(0) {
            assert!(
                legal
                    .abilities
                    .contains(&(land, baylee_engine::choice::GRANTED_ABILITY)),
                "the engine offers the granted ability the view described"
            );
            assert!(
                legal.mana_abilities.contains(&land),
                "and offers it as a mana ability, which is why it needs no stack"
            );
            return;
        }
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("seat 0 never got priority");
}

fn chromatic_lantern() -> CardIndex {
    by_oracle_id("539f5396-d99a-417d-a84c-dff7930b5900")
        .expect("Chromatic Lantern is in the pool")
        .index
}

fn machine_gods_effigy() -> CardIndex {
    by_oracle_id("64ebdd6f-acde-4aab-a86b-2798bad5f70c")
        .expect("Machine God's Effigy is in the pool")
        .index
}

/// The first grant `card`'s front face writes, and which of its
/// abilities writes it.
fn first_grant(card: CardIndex) -> (u32, baylee_cards_dsl::Modifier) {
    let abilities = baylee_cards::by_index(card)
        .expect("in the pool")
        .abilities_for_face(0);
    abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| {
            let (_, grant) = baylee_cards::lines::grants_in(ability, &mut 0)
                .into_iter()
                .next()?;
            Some((u32::try_from(index).ok()?, *grant))
        })
        .expect("the card grants an ability")
}

/// Makes `source` grant `grant` to `target`, as an effect that resolved
/// would, through the dev door the test preset opens.
fn grant_from(
    engine: &mut Engine<Registry>,
    source: ObjectId,
    target: ObjectId,
    grant: baylee_cards_dsl::Modifier,
    timestamp: u64,
) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the test preset grants dev commands");
    let filter = baylee_engine::effects::EffectFilter::object(state, target);
    state
        .effects
        .register(baylee_engine::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(source),
            controller: PlayerId::new(0),
            origin: baylee_engine::effects::EffectOrigin::Resolution,
            layer: grant.layer(),
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier: grant,
        });
}

/// The one object of `card` in `zone`.
fn lying_in(state: &GameState, zone: ZoneLocation, card: Option<CardIndex>) -> ObjectId {
    state
        .zones
        .list(zone)
        .iter()
        .copied()
        .find(|&id| {
            card.is_none_or(|card| {
                state
                    .object(id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == card)
            })
        })
        .unwrap_or_else(|| panic!("nothing in {zone:?}"))
}

/// What `seat` is told about who granted `permanent`'s abilities.
fn grants_seen(
    engine: &Engine<Registry>,
    seat: PlayerId,
    permanent: ObjectId,
) -> Vec<baylee_view::GrantSource> {
    player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[])
        .battlefield
        .iter()
        .find(|o| o.id == permanent)
        .expect("on the shared battlefield")
        .grants
        .clone()
}

/// #212. A land under a Chromatic Lantern taps for an ability printed on
/// no card it has, and the view says whose sentence it is: the Lantern,
/// its front face, and the line of it that grants the `{T}`. A client
/// then draws the Lantern's words on the land's row, in the player's
/// language, instead of a label of its own.
///
/// Both seats are told, since the Lantern is on the battlefield. The
/// opponent's land and the Lantern itself are granted nothing: the grant
/// is "lands *you* control", and the Lantern's own `{T}` is printed on it.
#[test]
fn a_granted_ability_names_its_grantor_and_the_sentence_that_grants_it() {
    let card = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![card(island()), card(chromatic_lantern())];
    preset.seats[1].starting_battlefield = vec![card(island())];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    settle(&mut engine, None);

    let state = engine.state();
    let lantern = lying_in(state, ZoneLocation::Battlefield, Some(chromatic_lantern()));
    let land_of = |seat: PlayerId| {
        state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|&id| {
                state.object(id).is_some_and(|o| {
                    o.controller == seat && o.card.is_some_and(|c| c.index == island())
                })
            })
            .expect("each seat has its Island")
    };
    let (mine, theirs) = (land_of(me), land_of(them));

    let named = vec![baylee_view::GrantSource {
        source: Some(lantern),
        rules: Some(baylee_view::RulesFace {
            card: chromatic_lantern(),
            face: 0,
        }),
        text: Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 2,
        }),
    }];
    assert_eq!(grants_seen(&engine, me, mine), named);
    assert_eq!(
        grants_seen(&engine, them, mine),
        named,
        "the Lantern is public, so the opponent is told the same"
    );
    assert!(grants_seen(&engine, me, theirs).is_empty());
    assert!(grants_seen(&engine, me, lantern).is_empty());
}

/// #212, the hidden-information half. A grantor that lies where this
/// seat cannot look is not named at all.
///
/// A nontoken card keeps its handle across zones (#240), so the handle
/// would say which card in that hand or library granted it, and the
/// sentence would say what the card is. Seat 1's own hand is a place
/// seat 1 may look, so it is told about its Lantern; nobody is told about
/// a card in a library. The Lantern is not even asked for text on seat
/// 0's behalf ([`PlayerView::cards`]).
#[test]
fn a_grantor_this_seat_cannot_see_is_not_named() {
    let card = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![card(island())];
    preset.seats[1].starting_hand = Some(vec![card(chromatic_lantern())]);
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    settle(&mut engine, None);

    let state = engine.state();
    let land = lying_in(state, ZoneLocation::Battlefield, Some(island()));
    let in_their_hand = lying_in(state, ZoneLocation::Hand(them), Some(chromatic_lantern()));
    let in_my_library = lying_in(state, ZoneLocation::Library(me), None);
    let (_, grant) = first_grant(chromatic_lantern());
    grant_from(&mut engine, in_their_hand, land, grant, 1_000);
    grant_from(&mut engine, in_my_library, land, grant, 1_001);

    assert_eq!(
        grants_seen(&engine, me, land),
        vec![baylee_view::GrantSource::default(); 2],
        "seat 0 may see neither grantor"
    );
    let mine = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
    assert!(
        !mine.cards().any(|c| c == chromatic_lantern()),
        "and is not handed the Lantern's text by another door"
    );
    assert_eq!(
        grants_seen(&engine, them, land),
        vec![
            baylee_view::GrantSource {
                source: Some(in_their_hand),
                rules: Some(baylee_view::RulesFace {
                    card: chromatic_lantern(),
                    face: 0,
                }),
                text: Some(baylee_view::StackText {
                    face: 0,
                    line: 0,
                    of: 2,
                }),
            },
            baylee_view::GrantSource::default(),
        ],
        "seat 1 holds the Lantern; nobody may look into seat 0's library"
    );
}

/// #212. A grant whose source wrote nothing equal to it names the source
/// and no sentence: there is no nearest match.
///
/// The source is an Opt that has resolved into its owner's graveyard,
/// the shape of a spell that granted something until end of turn and
/// is gone. A graveyard is public, so the Opt is named. Opt writes no
/// grant at all, so a lookup that fell back to some sentence would print
/// Opt's scry on the land's row, and one that panicked on a spell would
/// take the game down.
#[test]
fn a_grant_its_source_never_wrote_names_no_sentence() {
    let card = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![card(opt())]);
    preset.seats[0].starting_battlefield = vec![card(island()); 2];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let view = settle(&mut engine, None);
    let opt = view
        .hand
        .iter()
        .find(|c| c.name == "Opt")
        .expect("Opt in hand")
        .id;
    let islands: Vec<ObjectId> = view.battlefield_of(me).map(|o| o.id).collect();
    engine
        .apply(me, PlayerAction::ActivateManaAbility { source: islands[0] })
        .expect("an Island taps for blue");
    engine
        .apply(me, PlayerAction::CastSpell { card: opt })
        .expect("the mana for it is floating");
    settle(&mut engine, None);
    assert_eq!(
        engine.state().object(opt).map(|o| o.zone),
        Some(Zone::Graveyard),
        "Opt resolved"
    );

    let (_, grant) = first_grant(chromatic_lantern());
    grant_from(&mut engine, opt, islands[1], grant, 1_000);
    let named_only = vec![baylee_view::GrantSource {
        source: Some(opt),
        rules: None,
        text: None,
    }];
    assert_eq!(grants_seen(&engine, me, islands[1]), named_only);
    assert_eq!(grants_seen(&engine, them, islands[1]), named_only);
}

/// A copiable exception keeps its original printed clause as provenance.
#[test]
fn a_copys_own_grant_is_read_off_the_card_it_is() {
    use baylee_engine::copiable_abilities::compose;
    use baylee_engine::object::AbilityList;
    let card = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let me = PlayerId::new(0);
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![card(machine_gods_effigy())];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    settle(&mut engine, None);
    let effigy = lying_in(
        engine.state(),
        ZoneLocation::Battlefield,
        Some(machine_gods_effigy()),
    );
    let lantern = baylee_cards::by_index(chromatic_lantern()).unwrap();
    let definition = baylee_cards::by_index(machine_gods_effigy()).unwrap();
    let own = AbilityList::from_static(
        definition.abilities_for_face(0),
        PrintedFace::new(definition.index, 0),
        None,
    );
    let target = AbilityList::from_static(
        lantern.abilities_for_face(0),
        PrintedFace::new(lantern.index, 0),
        None,
    );
    let AbilityDef::CopyOnEnter { mods, .. } = own.abilities[0] else {
        panic!("copy clause");
    };
    let blue_index = u32::try_from(target.abilities.len()).unwrap();
    let copied = compose(target, &own, 0, mods, None);
    engine
        .dev_state_mut(me)
        .unwrap()
        .object_mut(effigy)
        .unwrap()
        .take_abilities(copied.clone());
    let view = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
    let copy = view.battlefield.iter().find(|o| o.id == effigy).unwrap();
    assert_eq!(
        copy.rules,
        Some(baylee_view::RulesFace {
            card: chromatic_lantern(),
            face: 0
        })
    );
    assert!(
        copy.grants.is_empty(),
        "the copied blue ability is not an external modifier grant"
    );
    let mut state = engine.state().clone();
    let quoted = stacked(&mut state, machine_gods_effigy(), blue_index, copied);
    let Some(baylee_view::StackItem::Ability {
        ability,
        rules,
        text,
        ..
    }) = stack_item(&quoted)
    else {
        panic!("blue ability");
    };
    assert_eq!(
        ability,
        Some(baylee_core::ids::AbilityRef::new(machine_gods_effigy(), 0))
    );
    let face = PrintedFace::new(machine_gods_effigy(), 0).unwrap();
    assert_eq!(rules, Some(rules_face(face)));
    assert_eq!(
        text,
        stack_text(face, 0),
        "the quoted ability names Effigy's clause, not Lantern's"
    );
}

/// Forgotten Monument grants its other Caves `{T}`, pay 1 life: add one
/// mana of any colour — and the view says **nothing** about it, on
/// purpose.
///
/// [`baylee_view::GrantedMana`] carries a slot, the colours and the
/// amount and has no field for a price, because an ability printed on no
/// card has nowhere else to put one. Reporting this grant would therefore
/// tell a planner it may tap the Cave for free; it would tap it, and the
/// life would never be offered to pay. Saying nothing costs the planner
/// one land and costs the player nothing at all, which is the asymmetry
/// `baylee_cards_dsl::tap_only` encodes.
///
/// The second assertion is what keeps the first from being a way to hide
/// a broken grant: the engine still **offers** the ability under
/// `GRANTED_ABILITY`, so what the view withholds is a claim, not the
/// player's button.
#[test]
fn granted_mana_refuses_a_priced_grant() {
    use baylee_engine::choice::{Pending, PlayerAction};

    let monument = by_oracle_id("71393988-ad6f-43fd-9978-c0de15ae8e87")
        .expect("Forgotten Monument is in the pool")
        .index;
    let maw = by_oracle_id("952ab8fe-f7d3-4673-89de-8c6d3f8a081f")
        .expect("Cavernous Maw is in the pool")
        .index;
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![
        DeckEntry {
            card: maw,
            print: PrintRef::new(0),
        },
        DeckEntry {
            card: monument,
            print: PrintRef::new(0),
        },
    ];

    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let view = player_view(
        engine.state(),
        PlayerId::new(0),
        1,
        None,
        &SeatContext::default(),
        &[],
    );
    let cave = view
        .battlefield
        .iter()
        .find(|o| o.card.is_some_and(|c| c.index == maw))
        .expect("the Cave is on the battlefield");
    assert!(
        cave.granted_mana.is_none(),
        "the grant charges a life beside its {{T}} and `GrantedMana` has \
         nowhere to say so, so the view must withhold it rather than \
         report mana the engine will not hand over for nothing"
    );

    for _ in 0..30 {
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            break;
        };
        if player == PlayerId::new(0) {
            assert!(
                legal
                    .abilities
                    .contains(&(cave.id, baylee_engine::choice::GRANTED_ABILITY)),
                "the engine still offers the granted ability the view \
                 declined to describe: {:?}",
                legal.abilities
            );
            return;
        }
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("seat 0 never got priority");
}
