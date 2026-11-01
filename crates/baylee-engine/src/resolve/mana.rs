//! Mana production, and delayed mana triggers.
//!
//! One effect covers all of it. Which colors are available, how much mana
//! there is, and what it may be spent on are three independent questions,
//! and a card that answers them in a new combination needs no new code
//! here — only the color list is game state (commander identity, the lands
//! on the battlefield), so that is the one thing resolved on the spot.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

use baylee_cards_dsl::effect::{ManaDistribution, ManaRestriction, ManaSource};

mod production;

/// Executes one mana effect.
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    match op {
        Effect::AddManaUnlessAnyPlayerPays {
            source,
            amount,
            cost,
        } => {
            let options = resolving_colors(state, res, source);
            match options.as_slice() {
                [] => None,
                [only] => offer_prevention(state, res, *only, amount, cost, 0),
                _ => {
                    res.awaiting = Some(AwaitingOp::ManaUnlessPaidColor { amount, cost });
                    Some(Pending::ChooseColor {
                        player: res.controller,
                        options,
                    })
                }
            }
        }
        Effect::AddMana {
            source,
            amount,
            distribution,
            restriction,
        } => add_mana(state, res, source, &amount, distribution, restriction),
        Effect::ManaForColorsAmong(filter) => {
            let colors = eval::colors_among(state, res.controller, filter, res.rule_context());
            for color in colored(colors) {
                add(state, res, color, 1, None);
            }
            None
        }
        Effect::AddManaLikeEvent { amount } => add_like_event(state, res, amount),
        Effect::AddManaFor { who, color, amount } => {
            for player in players_of(who, state, res.controller, res) {
                add_to(state, res, player, color, amount, None);
            }
            None
        }
        Effect::AddManaAmountFor { who, color, amount } => {
            let you = res.controller;
            for player in players_of(who, state, you, res) {
                let n = u16::try_from(super::amount_for_player(&amount, state, you, res, player))
                    .unwrap_or(u16::MAX);
                add_to(state, res, player, color, n, None);
            }
            None
        }
        Effect::DelayedManaAtNextMain { color } => {
            // "That spell's mana value": the countered spell is in its
            // owner's graveyard by now, and is read as it was on the stack.
            let amount = u16::try_from(super::target_mana_value(res, state)).unwrap_or(u16::MAX);
            state.delayed.push(crate::state::DelayedTrigger {
                controller: res.controller,
                when: crate::state::DelayedWhen::NextMain,
                action: crate::state::DelayedAction::AddMana { color, amount },
            });
            None
        }
        _ => unreachable!("not a mana effect"),
    }
}

/// Produce ordinary mana whose lifetime extends through this combat.
pub(super) fn add_combat_mana(
    state: &mut GameState,
    res: &Resolution,
    color: ManaColor,
    amount: u16,
) {
    let production = state.next_restriction_id;
    add(state, res, color, amount, None);
    if state.next_restriction_id != production {
        state.players[usize::from(res.controller.get())]
            .mana_pool
            .retain_until_combat_end(baylee_core::mana::RestrictionId(production));
    }
}

/// Adds the mana, asking for a color first when there is more than one.
fn add_mana(
    state: &mut GameState,
    res: &mut Resolution,
    source: ManaSource,
    amount: &Amount,
    distribution: ManaDistribution,
    restriction: Option<ManaRestriction>,
) -> Option<Pending> {
    let you = res.controller;
    if matches!(amount, Amount::DevotionToManaColor) {
        let options = resolving_colors(state, res, source);
        let Some(amounts) = devotion(state, you) else {
            state.numeric_failure = Some("devotion exceeds u16 mana-production units");
            return None;
        };
        match options.as_slice() {
            [] => return None,
            [only] => {
                add(state, res, *only, amounts[only.index()], restriction);
                return None;
            }
            _ => {}
        }
        res.awaiting = Some(AwaitingOp::ManaChoice {
            recipient: you,
            colors: options.clone(),
            distinct: false,
            remaining: 1,
            per_pick: 0,
            by_color: Some(amounts),
            restriction,
        });
        return Some(Pending::ChooseColor {
            player: you,
            options,
        });
    }
    let Ok(n) = u16::try_from(amount2(amount, state, you, res)) else {
        state.numeric_failure = Some("one mana-production instruction exceeds u16 units");
        return None;
    };
    if n == 0 {
        return None;
    }
    let options = resolving_colors(state, res, source);
    match options[..] {
        [] => return None,
        [only] => {
            let amount = if distribution == ManaDistribution::DifferentColors {
                1
            } else {
                n
            };
            add(state, res, only, amount, restriction);
            return None;
        }
        _ => {}
    }
    let (picks, per_pick) = match distribution {
        ManaDistribution::OneColor => (1, n),
        ManaDistribution::AnyCombination => (n, 1),
        ManaDistribution::DifferentColors => (n.min(options.len() as u16), 1),
    };
    res.awaiting = Some(AwaitingOp::ManaChoice {
        recipient: you,
        colors: options.clone(),
        distinct: distribution == ManaDistribution::DifferentColors,
        remaining: picks,
        per_pick,
        by_color: None,
        restriction,
    });
    Some(Pending::ChooseColor {
        player: you,
        options,
    })
}

/// Adds the extra mana to the player who tapped the land, with their choice
/// restricted to types that activation actually produced.
fn add_like_event(state: &mut GameState, res: &mut Resolution, amount: u16) -> Option<Pending> {
    let event = res.event_mana?;
    let options: Vec<_> = ManaColor::ALL
        .into_iter()
        .filter(|color| event.types & (1 << (*color as u8)) != 0)
        .collect();
    if amount == 0 || options.is_empty() {
        return None;
    }
    if let [color] = options[..] {
        add_to(state, res, event.player, color, amount, None);
        return None;
    }
    res.awaiting = Some(AwaitingOp::ManaChoice {
        recipient: event.player,
        colors: options.clone(),
        distinct: false,
        remaining: 1,
        per_pick: amount,
        by_color: None,
        restriction: None,
    });
    Some(Pending::ChooseColor {
        player: event.player,
        options,
    })
}

/// Adds mana of one settled color, restricted or not, and journals it.
pub(super) fn add(
    state: &mut GameState,
    res: &Resolution,
    color: ManaColor,
    amount: u16,
    restriction: Option<ManaRestriction>,
) {
    add_to(state, res, res.controller, color, amount, restriction);
}

/// [`add`] into `you`'s pool, which is the ability's controller's unless
/// the effect names another player ("its controller adds", Gauntlet of
/// Might).
pub(super) fn add_to(
    state: &mut GameState,
    res: &Resolution,
    you: PlayerId,
    color: ManaColor,
    amount: u16,
    restriction: Option<ManaRestriction>,
) {
    if amount == 0 {
        return;
    }
    let color = res
        .subject
        .mana_tap_source()
        .and_then(|identity| state.damage_source(identity.object, Some(identity.version)))
        .map_or(color, |object| {
            crate::mana_rules::replace(state, object, color)
        });
    let obligation_before =
        (res.mana_ability && res.event_mana.is_none() && state.constrained_payment(you).is_some())
            .then(|| state.players[usize::from(you.get())].mana_pool.clone());
    let snow = state
        .damage_source(res.source, super::source_version(state, res))
        .is_some_and(|o| {
            o.characteristics()
                .supertypes
                .contains(baylee_core::types::SupertypeSet::SNOW)
        });
    let ordinary = restriction.is_none_or(|restriction| !restriction.restricts);
    if ordinary
        && state.players[you.get() as usize]
            .mana_pool
            .available(color)
            .checked_add(u32::from(amount))
            .is_none()
    {
        state.numeric_failure = Some("mana production exceeds u32 per color");
        return;
    }
    let provenance = crate::mana_provenance::ManaProvenance::capture(
        state,
        res.rule_context(),
        super::source_version(state, res),
        res.controller,
        restriction,
    );
    if !crate::mana_provenance::deposit(state, you, color, amount, snow, provenance) {
        return;
    }
    if let Some(before) = obligation_before {
        state.note_constrained_production(you, &before);
    }
    state.journal.record(GameEvent::ManaProduced {
        player: you,
        color,
        amount,
        source: Some(res.source),
    });
}

fn resolving_colors(state: &GameState, res: &Resolution, source: ManaSource) -> Vec<ManaColor> {
    if source == ManaSource::ExiledWithSource {
        return super::subjects::source(state, res).map_or_else(Vec::new, |identity| {
            production::exiled_colors_for(state, identity, super::subjects::acquisition(state, res))
        });
    }
    if source != ManaSource::CircledColors {
        return colors_of(state, res.controller, source, res.source);
    }
    let colors = if res.mana_ability {
        res.subject.circled_colors()
    } else {
        state
            .object(res.on_stack)
            .and_then(|object| {
                let index = object.ability?.index as usize;
                Some(
                    object
                        .own_abilities
                        .as_ref()?
                        .entries()
                        .get(index)?
                        .circled_colors,
                )
            })
            .unwrap_or(baylee_core::color::ColorSet::EMPTY)
    };
    colored(colors)
}

/// The colors this source can produce right now.
///
/// `from` is the permanent whose ability is making the mana, and it is a
/// parameter because two of the sources are answered by the object rather
/// than by the card: "one mana of the chosen color" is a different colour on
/// each Thriving Moor, and the card they share cannot say which.
///
/// Re-exported as `resolve::colors_of`, where the reason is written down.
pub fn colors_of(
    state: &GameState,
    you: PlayerId,
    source: ManaSource,
    from: ObjectId,
) -> Vec<ManaColor> {
    match source {
        ManaSource::IntrinsicBasicLandTypes => crate::casting::intrinsic_mana_colors(state, from),
        ManaSource::Fixed(color) => vec![color],
        ManaSource::Choice(colors) => colors.to_vec(),
        ManaSource::CommanderIdentity => {
            // The marker list, not the command zone: colour identity is a
            // property of the commander card wherever it is (CR 903.4), so
            // reading the zone made an Arcane Signet stop producing the
            // moment its commander was cast — exactly when it matters.
            let colors = state.commander_colors(you);
            // CR 903.4f: no commander supplies no color; a colorless commander
            // also supplies no color. Neither makes this instruction add {C}.
            colored(colors)
        }
        ManaSource::ColorsAmong { filter, zone } => colored(eval::colors_in_zone(
            filter,
            zone,
            state,
            you,
            eval::live_context(state, from),
        )),
        ManaSource::LandColor {
            filter,
            mine,
            any_type,
        } => production::land_colors(state, you, from, filter, mine, any_type),
        ManaSource::ExiledWithSource => production::exiled_colors(state, from),
        ManaSource::CircledColors => {
            let colors = state.printed_ability_list(from).map_or(
                baylee_core::color::ColorSet::EMPTY,
                |list| {
                    list.entries()
                        .iter()
                        .fold(baylee_core::color::ColorSet::EMPTY, |colors, entry| {
                            colors.union(entry.circled_colors)
                        })
                },
            );
            colored(colors)
        }
        ManaSource::Chosen => match state.object(from).and_then(|o| o.chosen_color) {
            // No colour was ever chosen — a copy that did not arrive
            // through the replacement, or a board built by a test. The
            // ability resolves and adds nothing, which is what an empty
            // option list means here.
            None => Vec::new(),
            Some(c) => vec![c],
        },
        ManaSource::ChosenOr(colors) => {
            let mut options = colors.to_vec();
            if let Some(c) = state.object(from).and_then(|o| o.chosen_color)
                && !options.contains(&c)
            {
                options.push(c);
            }
            options
        }
    }
}

/// The five colors of a [`ColorSet`], as mana.
fn colored(colors: ColorSet) -> Vec<ManaColor> {
    [
        (ManaColor::White, baylee_core::color::Color::White),
        (ManaColor::Blue, baylee_core::color::Color::Blue),
        (ManaColor::Black, baylee_core::color::Color::Black),
        (ManaColor::Red, baylee_core::color::Color::Red),
        (ManaColor::Green, baylee_core::color::Color::Green),
    ]
    .into_iter()
    .filter(|&(_, c)| colors.contains(c))
    .map(|(m, _)| m)
    .collect()
}

/// Count symbols once per color; a hybrid symbol contributes to both colors,
/// while generic, colorless, snow and variable symbols contribute to neither.
fn devotion(state: &GameState, you: PlayerId) -> Option<[u16; 6]> {
    let mut amounts = [0_u16; 6];
    for id in state.battlefield_seen() {
        let object = state.object(id)?;
        if object.controller != you {
            continue;
        }
        for symbol in object.characteristics().mana_cost.symbols() {
            for color in baylee_core::color::Color::ALL {
                if symbol.colors().contains(color) {
                    let slot = ManaColor::from_color(color).index();
                    amounts[slot] = amounts[slot].checked_add(1)?;
                }
            }
        }
    }
    Some(amounts)
}

/// Ask each live player in APNAP order, stopping as soon as somebody pays.
pub(super) fn offer_prevention(
    state: &mut GameState,
    res: &mut Resolution,
    color: ManaColor,
    amount: u32,
    cost: baylee_core::mana::ManaCost,
    next: usize,
) -> Option<Pending> {
    let seats = state.players.len();
    for offset in next..seats {
        let seat = (usize::from(state.turn.active.get()) + offset) % seats;
        let player = state.players[seat].id;
        if state.has_left(player) {
            continue;
        }
        res.awaiting = Some(AwaitingOp::PlayerMayPay {
            player,
            cost,
            outcome: ManaPaymentOutcome::PreventMana {
                color,
                amount,
                next: offset + 1,
            },
        });
        return Some(Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayMana { cost },
            source: resolving_ability(state, res),
        });
    }
    let Ok(amount) = u16::try_from(amount) else {
        state.numeric_failure = Some("one mana-production instruction exceeds u16 units");
        return None;
    };
    add(state, res, color, amount, None);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::{CardLookup, Commander};
    use baylee_core::color::Color;
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    use baylee_core::types::TypeSet;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }
    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                rule_print: None,
                choices: baylee_core::card_metadata::DeckChoices::default(),
                card: forest,
                print: baylee_core::ids::PrintRef::new(0),
            })
            .collect();
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 5,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn permanent(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    }

    /// A land on `owner`'s battlefield producing exactly `colors`.
    fn land(state: &mut GameState, owner: PlayerId, name: &str, colors: ColorSet) -> ObjectId {
        let id = permanent(state, owner, name);
        let base = state.object_mut(id).expect("just made it").base_mut();
        base.types = TypeSet::LAND;
        base.produced_colors = colors;
        for (color, subtype) in [
            (Color::White, baylee_core::generated::subtypes::land::PLAINS),
            (Color::Blue, baylee_core::generated::subtypes::land::ISLAND),
            (Color::Black, baylee_core::generated::subtypes::land::SWAMP),
            (Color::Red, baylee_core::generated::subtypes::land::MOUNTAIN),
            (Color::Green, baylee_core::generated::subtypes::land::FOREST),
        ] {
            if colors.contains(color) {
                base.subtypes.insert(subtype);
            }
        }
        id
    }

    fn give_colorless_mana_ability(state: &mut GameState, land: ObjectId) {
        static ABILITIES: &[baylee_cards_dsl::AbilityDef] =
            &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
                ManaColor::Colorless,
                1
            )])];
        state.object_mut(land).unwrap().own_abilities =
            Some(crate::object::AbilityList::from_static(ABILITIES, None, None).into_bundle());
    }

    /// The three sources that read nothing but their own argument, and the
    /// one that is a list of them. `Choice` keeps the card's order rather
    /// than sorting into WUBRG: the options are offered to a player, and a
    /// reordered list is a different keystroke for the same card.
    #[test]
    fn a_fixed_or_listed_source_is_its_own_answer() {
        let mut state = state();
        let rock = permanent(&mut state, me(), "Rock");
        assert_eq!(
            colors_of(&state, me(), ManaSource::Fixed(ManaColor::Red), rock),
            vec![ManaColor::Red]
        );
        assert_eq!(
            colors_of(
                &state,
                me(),
                ManaSource::Choice(&[ManaColor::Green, ManaColor::White]),
                rock
            ),
            vec![ManaColor::Green, ManaColor::White]
        );
    }

    /// CR 903.4: colour identity is a property of the commander card
    /// wherever it is. Reading the command zone instead made an Arcane
    /// Signet stop producing the moment its commander was cast — exactly
    /// when it matters — so this test puts the commander on the
    /// **battlefield** and asks anyway.
    #[test]
    fn a_commanders_identity_is_read_from_the_marker_and_not_from_its_zone() {
        let mut state = state();
        let signet = permanent(&mut state, me(), "Arcane Signet");
        let general = permanent(&mut state, me(), "General");
        state
            .object_mut(general)
            .expect("just made it")
            .base_mut()
            .color_identity = ColorSet::of(Color::White).union(ColorSet::of(Color::Blue));

        assert_eq!(
            colors_of(&state, me(), ManaSource::CommanderIdentity, signet),
            Vec::<ManaColor>::new(),
            "no commander means no color to produce"
        );

        state.commanders[me().get() as usize].push(Commander {
            object: general,
            casts: 1,
            answered: 0,
        });
        assert_eq!(
            colors_of(&state, me(), ManaSource::CommanderIdentity, signet),
            vec![ManaColor::White, ManaColor::Blue],
            "the marker answers wherever the card is, and it is not in the \
             command zone here"
        );
        assert_eq!(
            colors_of(&state, them(), ManaSource::CommanderIdentity, signet),
            Vec::<ManaColor>::new(),
            "and it is this player's commander, not the table's"
        );
    }

    /// The union over one side of the table, which is the whole of
    /// Reflecting Pool and Exotic Orchard. Three things it must not do:
    /// count a non-land, count the other side's lands, or lose the
    /// colorless half.
    #[test]
    fn land_colours_are_the_union_of_the_side_that_was_asked_about() {
        let mut state = state();
        let pool = land(&mut state, me(), "Reflecting Pool", ColorSet::EMPTY);
        land(&mut state, me(), "Forest", ColorSet::of(Color::Green));
        land(&mut state, me(), "Plains", ColorSet::of(Color::White));
        land(&mut state, them(), "Mountain", ColorSet::of(Color::Red));
        let rock = permanent(&mut state, me(), "Signet");
        state
            .object_mut(rock)
            .expect("just made it")
            .base_mut()
            .produced_colors = ColorSet::of(Color::Black);

        assert_eq!(
            colors_of(
                &state,
                me(),
                ManaSource::LandColor {
                    filter: &baylee_cards_dsl::Filter::Any,
                    mine: true,
                    any_type: true
                },
                pool
            ),
            vec![ManaColor::White, ManaColor::Green],
            "my lands, in WUBRG order, and the Signet is not a land"
        );
        assert_eq!(
            colors_of(
                &state,
                me(),
                ManaSource::LandColor {
                    filter: &baylee_cards_dsl::Filter::Any,
                    mine: false,
                    any_type: false
                },
                pool
            ),
            vec![ManaColor::Red],
            "an Exotic Orchard reads the other side of the table"
        );

        state
            .object_mut(pool)
            .expect("still there")
            .base_mut()
            .produced_colorless = true;
        give_colorless_mana_ability(&mut state, pool);
        assert_eq!(
            colors_of(
                &state,
                me(),
                ManaSource::LandColor {
                    filter: &baylee_cards_dsl::Filter::Any,
                    mine: true,
                    any_type: true
                },
                pool
            ),
            vec![ManaColor::White, ManaColor::Green, ManaColor::Colorless],
            "a land making colorless mana is a type this can produce, and \
             it is not in the ColorSet the other four came out of"
        );
    }

    /// "Any color" is not "any type": colorless mana is a type of mana and
    /// not a colour (CR 106.1a, CR 106.1b). Exotic Orchard and Fellwar Stone
    /// across a Wastes used to offer `{C}`, because the union kept the
    /// colorless half for both sentences.
    #[test]
    fn any_color_a_land_could_produce_is_never_colorless() {
        let mut state = state();
        let orchard = land(&mut state, me(), "Exotic Orchard", ColorSet::EMPTY);
        land(&mut state, them(), "Island", ColorSet::of(Color::Blue));
        let wastes = land(&mut state, them(), "Wastes", ColorSet::EMPTY);
        state
            .object_mut(wastes)
            .expect("just made it")
            .base_mut()
            .produced_colorless = true;
        give_colorless_mana_ability(&mut state, wastes);

        let color = ManaSource::LandColor {
            filter: &baylee_cards_dsl::Filter::Any,
            mine: false,
            any_type: false,
        };
        let kind = ManaSource::LandColor {
            filter: &baylee_cards_dsl::Filter::Any,
            mine: false,
            any_type: true,
        };
        assert_eq!(
            colors_of(&state, me(), color, orchard),
            vec![ManaColor::Blue],
            "any color: the Wastes' colorless mana is no colour"
        );
        assert_eq!(
            colors_of(&state, me(), kind, orchard),
            vec![ManaColor::Blue, ManaColor::Colorless],
            "any type: it is a type"
        );
    }

    /// `produced_colors` is a reading of the **card**, and a land whose mana
    /// is "the chosen color" has none there to read — the answer is on the
    /// object. Without this a Reflecting Pool beside an Uncharted Haven saw
    /// a land that makes nothing.
    #[test]
    fn a_land_making_the_chosen_colour_is_read_off_the_object() {
        static CHOSEN: &[baylee_cards_dsl::AbilityDef] =
            &[baylee_cards_dsl::mana_ability!(&[Effect::mana_chosen()])];
        let mut state = state();
        let pool = land(&mut state, me(), "Reflecting Pool", ColorSet::EMPTY);
        let haven = land(&mut state, me(), "Uncharted Haven", ColorSet::EMPTY);
        state
            .object_mut(haven)
            .expect("just made it")
            .base_mut()
            .produced_chosen = true;
        state.object_mut(haven).unwrap().own_abilities =
            Some(crate::object::AbilityList::from_static(CHOSEN, None, None).into_bundle());

        assert!(
            colors_of(
                &state,
                me(),
                ManaSource::LandColor {
                    filter: &baylee_cards_dsl::Filter::Any,
                    mine: true,
                    any_type: true
                },
                pool
            )
            .is_empty(),
            "nothing has been chosen yet, so the card is right that it \
             produces nothing"
        );

        state.object_mut(haven).expect("still there").chosen_color = Some(ManaColor::Blue);
        assert_eq!(
            colors_of(
                &state,
                me(),
                ManaSource::LandColor {
                    filter: &baylee_cards_dsl::Filter::Any,
                    mine: true,
                    any_type: true
                },
                pool
            ),
            vec![ManaColor::Blue]
        );
    }

    /// The two sources that ask the object they are on. `Chosen` with
    /// nothing chosen is an empty list rather than a guess — a copy that did
    /// not arrive through the replacement, or a board built by a test; the
    /// ability resolves and adds nothing. `ChosenOr` adds the chosen colour
    /// to the printed list, and adds it once.
    #[test]
    fn a_chosen_colour_is_asked_of_the_permanent_that_is_making_the_mana() {
        let mut state = state();
        let one = permanent(&mut state, me(), "Thriving Moor");
        let two = permanent(&mut state, me(), "Another Moor");

        assert!(
            colors_of(&state, me(), ManaSource::Chosen, one).is_empty(),
            "no colour was ever chosen"
        );

        state.object_mut(one).expect("still there").chosen_color = Some(ManaColor::Red);
        state.object_mut(two).expect("still there").chosen_color = Some(ManaColor::Green);
        assert_eq!(
            colors_of(&state, me(), ManaSource::Chosen, one),
            vec![ManaColor::Red]
        );
        assert_eq!(
            colors_of(&state, me(), ManaSource::Chosen, two),
            vec![ManaColor::Green],
            "the two share a card and cannot both read it for this"
        );

        let printed = &[ManaColor::Black, ManaColor::Red];
        assert_eq!(
            colors_of(&state, me(), ManaSource::ChosenOr(printed), one),
            vec![ManaColor::Black, ManaColor::Red],
            "the chosen colour is already printed, so it is not offered twice"
        );
        assert_eq!(
            colors_of(&state, me(), ManaSource::ChosenOr(printed), two),
            vec![ManaColor::Black, ManaColor::Red, ManaColor::Green]
        );
    }
}
