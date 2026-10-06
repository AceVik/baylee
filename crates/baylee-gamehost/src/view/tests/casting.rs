use super::*;

fn snapcaster_mage() -> CardIndex {
    by_oracle_id("2bb2eda7-3b38-4c56-870f-c3218a1056f5")
        .unwrap()
        .index
}

/// What `seat` is told it may pay to cast `card` from the graveyard.
fn flashback_for(engine: &Engine<Registry>, seat: PlayerId, card: ObjectId) -> Option<ManaCost> {
    let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);
    view.graveyards
        .iter()
        .flatten()
        .find(|o| o.id == card)
        .unwrap_or_else(|| panic!("seat {seat:?} sees the card in a graveyard"))
        .flashback
}

/// #242. A card this seat may cast from its graveyard says so, at the
/// price the cast will charge, and says so to that seat alone.
///
/// The engine names a graveyard spell in `LegalActions::castable` only
/// once its cost is already floating, so a planner that walked its hand
/// never tapped for one: Snapcaster Mage gave Opt flashback, and Opt
/// stayed where it was beside an untapped Island. Played, not built:
/// Opt is cast and resolves, then the Mage enters and targets it.
///
/// The opponent sees the same Opt in the same graveyard and is told
/// nothing, because it may not cast it (`casting::can_cast` asks for the
/// caster's own graveyard); and nobody is told anything once the grant
/// has ended with the turn.
#[test]
fn a_granted_flashback_is_shown_to_the_seat_that_may_cast_it() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let card = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    let printed = baylee_cards::by_index(opt()).expect("Opt").faces[0].mana_cost;
    preset.seats[0].starting_hand = Some(vec![card(opt()), card(snapcaster_mage())]);
    preset.seats[0].starting_battlefield = vec![card(island()); 3];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");

    let view = settle(&mut engine, None);
    let in_hand = |name: &str| {
        view.hand
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} in hand"))
            .id
    };
    let (opt, mage) = (in_hand("Opt"), in_hand("Snapcaster Mage"));
    let islands: Vec<ObjectId> = view.battlefield_of(me).map(|o| o.id).collect();
    let cast = |engine: &mut Engine<Registry>, lands: &[ObjectId], card: ObjectId| {
        for &source in lands {
            engine
                .apply(me, PlayerAction::ActivateManaAbility { source })
                .expect("an Island taps for blue");
        }
        engine
            .apply(me, PlayerAction::CastSpell { card })
            .expect("the mana for it is floating");
    };

    cast(&mut engine, &islands[..1], opt);
    settle(&mut engine, None);
    assert_eq!(
        (
            flashback_for(&engine, me, opt),
            flashback_for(&engine, them, opt)
        ),
        (None, None),
        "Opt in the graveyard before the grant is castable by nobody"
    );

    cast(&mut engine, &islands[1..], mage);
    settle(&mut engine, Some(opt));
    assert_eq!(
        flashback_for(&engine, me, opt),
        Some(printed),
        "the Mage's grant costs Opt's own mana cost, and the owner is told"
    );
    assert_eq!(
        flashback_for(&engine, them, opt),
        None,
        "the opponent may not cast a card out of somebody else's graveyard"
    );

    engine.apply(me, PlayerAction::PassPriority).unwrap();
    settle(&mut engine, None);
    assert_eq!(
        flashback_for(&engine, me, opt),
        None,
        "until end of turn: seat 0's next main phase has no grant left"
    );
}

/// Muldrotha's door to [`PublicObject::flashback`]: a permanent card its
/// owner may cast from the graveyard is priced at its own mana cost, to
/// its owner alone, and only while the allowance lasts. Without it the
/// client's planner never tapped for a graveyard creature, because the
/// engine offers it in `castable` only once the mana is floating (#242's
/// reason, one permission further on).
#[test]
fn a_graveyard_permission_is_priced_for_the_owner_while_it_lasts() {
    use baylee_engine::{event::Cause, zone::ZonePosition};
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let entry = |name| DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![entry("Muldrotha, the Gravetide")];
    preset.seats[0].starting_hand = Some(vec![entry("Llanowar Elves")]);
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let view = settle(&mut engine, None);
    let elf = view
        .hand
        .iter()
        .find(|o| o.name == "Llanowar Elves")
        .expect("the Elves in hand")
        .id;
    engine
        .dev_state_mut(me)
        .unwrap()
        .move_object(
            elf,
            ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    let printed = baylee_cards::by_index(entry("Llanowar Elves").card)
        .expect("Llanowar Elves")
        .faces[0]
        .mana_cost;
    assert_eq!(
        (
            flashback_for(&engine, me, elf),
            flashback_for(&engine, them, elf)
        ),
        (Some(printed), None),
        "the owner is told the Elves' own cost, the opponent nothing"
    );

    for _ in 0..40 {
        if engine.state().turn.active == them {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected question: {other:?}"),
        }
    }
    assert_eq!(
        flashback_for(&engine, me, elf),
        None,
        "\"during each of your turns\": not on the opponent's"
    );
}

/// Escape's door to [`PublicObject::flashback`]: Uro in its owner's
/// graveyard is priced at its escape mana once five *other* cards lie
/// beside it, and at nothing while there are four, because the cast
/// cannot be paid (CR 702.138a). To its owner alone.
#[test]
fn an_escape_is_priced_once_the_graveyard_can_pay_it() {
    use baylee_engine::{event::Cause, zone::ZonePosition};
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let entry = |name| DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    let mut hand = vec![entry("Uro, Titan of Nature's Wrath")];
    hand.extend(std::iter::repeat_n(entry("Llanowar Elves"), 5));
    preset.seats[0].starting_hand = Some(hand);
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let view = settle(&mut engine, None);
    let uro = view
        .hand
        .iter()
        .find(|o| o.name == "Uro, Titan of Nature's Wrath")
        .expect("Uro in hand")
        .id;
    let elves: Vec<ObjectId> = view
        .hand
        .iter()
        .filter(|o| o.name == "Llanowar Elves")
        .map(|o| o.id)
        .collect();
    assert_eq!(elves.len(), 5);
    let bury = |engine: &mut Engine<Registry>, card| {
        engine
            .dev_state_mut(me)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Graveyard(me),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        engine.refresh_offer();
    };
    bury(&mut engine, uro);
    for &elf in &elves[..4] {
        bury(&mut engine, elf);
    }
    assert_eq!(
        flashback_for(&engine, me, uro),
        None,
        "four other cards cannot pay an escape that exiles five"
    );
    bury(&mut engine, elves[4]);
    assert_eq!(
        (
            flashback_for(&engine, me, uro),
            flashback_for(&engine, them, uro)
        ),
        (Some(ManaCost::parse("{G}{G}{U}{U}")), None),
        "the escape's mana to its owner, nothing to the opponent"
    );
}

/// The other door to [`PublicObject::flashback`]: a *printed* flashback
/// is priced at what the card prints, not at its mana cost.
///
/// This was pinned shut while no face that prints the keyword had it
/// written; Memory Deluge is the first, `{2}{U}{U}` in front of a
/// `{5}{U}{U}` flashback, and a view that said four would have a planner
/// tap four lands for a cast the engine then refuses. Played, not
/// built: the Deluge is cast off four Islands and resolves into the
/// graveyard, where its owner alone is told the flashback price.
#[test]
fn a_printed_flashback_is_priced_at_its_printed_cost() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let deluge = by_oracle_id("e6fd55f2-7e26-469c-a44a-ea2eb90e19a9")
        .unwrap()
        .index;
    let entry = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![entry(deluge)]);
    preset.seats[0].starting_battlefield = vec![entry(island()); 4];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");

    let view = settle(&mut engine, None);
    let card = view
        .hand
        .iter()
        .find(|c| c.name == "Memory Deluge")
        .unwrap()
        .id;
    for source in view.battlefield_of(me).map(|o| o.id).collect::<Vec<_>>() {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source })
            .expect("an Island taps for blue");
    }
    engine
        .apply(me, PlayerAction::CastSpell { card })
        .expect("the mana for it is floating");
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: options[..usize::from(min)].to_vec(),
                        },
                    )
                    .expect("two of the four looked at");
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question: {other:?}"),
        }
    }
    settle(&mut engine, None);
    assert_eq!(
        flashback_for(&engine, me, card),
        Some(baylee_core::mana::ManaCost::parse("{5}{U}{U}")),
        "the printed flashback cost, not the mana cost"
    );
    assert_eq!(
        flashback_for(&engine, them, card),
        None,
        "to its owner alone"
    );
}

fn cavern_of_souls() -> CardIndex {
    by_oracle_id("89ca686a-7c72-4d8f-9290-e89635624a83")
        .unwrap()
        .index
}

fn sea_eagle() -> CardIndex {
    by_oracle_id("acb57162-7093-4a3c-9818-d3b61ce757c6")
        .unwrap()
        .index
}

/// Whether `seat` is told the object can't be countered.
fn uncounterable_to(engine: &Engine<Registry>, seat: PlayerId, id: ObjectId) -> bool {
    let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);
    let object = view.object(id).expect("the object is in public view");
    object.keywords & baylee_cards::dsl::KeywordSet::UNCOUNTERABLE.bits() != 0
}

/// #243. A spell that can't be countered says so on the stack, to every
/// seat, even when no printed word makes it so.
///
/// The printed "can't be countered" rode the projected keywords all
/// along. The Cavern of Souls kind is a rider on the spell that no
/// characteristic carries, so a creature cast with that mana looked
/// counterable to every seat, the house AI among them. The bit is now
/// set from `GameObject::can_be_countered`, the predicate the counter
/// itself asks.
///
/// The same card is cast twice, once off Islands and once off the Cavern,
/// so only the mana differs between the two answers. The Cavern's Eagle
/// then loses the bit once it is a permanent, because the rider belongs
/// to the spell.
#[test]
fn a_spell_that_cannot_be_countered_says_so_on_the_stack() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let card = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![card(sea_eagle()), card(sea_eagle())]);
    preset.seats[0].starting_battlefield = vec![
        card(cavern_of_souls()),
        card(island()),
        card(island()),
        card(island()),
    ];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");

    let view = settle(&mut engine, None);
    let eagles: Vec<ObjectId> = view.hand.iter().map(|c| c.id).collect();
    let lands = |name: &str| -> Vec<ObjectId> {
        view.battlefield_of(me)
            .filter(|o| o.name == name)
            .map(|o| o.id)
            .collect()
    };
    let (cavern, islands) = (lands("Cavern of Souls")[0], lands("Island"));

    for &source in &islands[..2] {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine
        .apply(me, PlayerAction::CastSpell { card: eagles[0] })
        .expect("two Islands pay {1}{U}");
    assert!(
        !uncounterable_to(&engine, me, eagles[0]) && !uncounterable_to(&engine, them, eagles[0]),
        "an Eagle paid for with Islands can be countered, and nobody is told otherwise"
    );
    settle(&mut engine, None);

    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: cavern,
                ability_index: 1,
            },
        )
        .expect("the Cavern's restricted mana");
    engine
        .apply(me, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    engine
        .apply(me, PlayerAction::ActivateManaAbility { source: islands[2] })
        .unwrap();
    engine
        .apply(me, PlayerAction::CastSpell { card: eagles[1] })
        .expect("the Cavern's blue and an Island pay {1}{U}");
    assert!(
        engine
            .state()
            .object(eagles[1])
            .is_some_and(|o| !o.can_be_countered()),
        "the Cavern's mana paid for this one, so the engine will not counter it"
    );
    assert!(
        uncounterable_to(&engine, me, eagles[1]) && uncounterable_to(&engine, them, eagles[1]),
        "and every seat is told so while it is on the stack"
    );

    settle(&mut engine, None);
    let eagle = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[])
        .battlefield_of(me)
        .filter(|o| o.name == "Sea Eagle")
        .map(|o| o.id)
        .max()
        .expect("the second Eagle has resolved");
    assert!(
        !uncounterable_to(&engine, me, eagle),
        "the rider belongs to the spell, and the permanent it became is not a spell"
    );
}
