//! Mana production, and delayed mana triggers.
//!
//! One effect covers all of it. Which colors are available, how much mana
//! there is, and what it may be spent on are three independent questions,
//! and a card that answers them in a new combination needs no new code
//! here — only the color list is game state (commander identity, the lands
//! on the battlefield), so that is the one thing resolved on the spot.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

use baylee_cards_dsl::effect::{ManaRestriction, ManaSource};

/// Executes one mana effect.
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    match op {
        Effect::AddMana {
            source,
            amount,
            combination,
            restriction,
        } => add_mana(state, res, source, &amount, combination, restriction),
        Effect::DelayedManaAtNextFirstMain { color } => {
            let cmc = res
                .targets
                .first()
                .and_then(|t| state.object(*t))
                .map_or(0, |o| o.characteristics().mana_cost.cmc());
            state.delayed.push(crate::state::DelayedTrigger {
                controller: res.controller,
                when: crate::state::DelayedWhen::NextFirstMain,
                action: crate::state::DelayedAction::AddMana {
                    color,
                    amount: cmc as u16,
                },
            });
            None
        }
        _ => unreachable!("not a mana effect"),
    }
}

/// Adds the mana, asking for a color first when there is more than one.
fn add_mana(
    state: &mut GameState,
    res: &mut Resolution,
    source: ManaSource,
    amount: &Amount,
    combination: bool,
    restriction: Option<ManaRestriction>,
) -> Option<Pending> {
    let you = res.controller;
    let n = amount2(amount, state, you, res) as u16;
    if n == 0 {
        return None;
    }
    let options = colors_of(state, you, source, res.source);
    match options[..] {
        [] => return None,
        [only] => {
            add(state, res, only, n, restriction);
            return None;
        }
        _ => {}
    }
    // "In any combination of colors" is one pick per mana; anything else
    // picks one color for the whole amount.
    let (picks, per_pick) = if combination { (n, 1) } else { (1, n) };
    res.awaiting = Some(AwaitingOp::ManaChoice {
        colors: options.clone(),
        remaining: picks,
        per_pick,
        restriction,
    });
    Some(Pending::ChooseColor {
        player: you,
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
    let you = res.controller;
    if let Some(ManaRestriction { filter, rider }) = restriction {
        let id = state.next_restriction_id;
        state.next_restriction_id += 1;
        state
            .restriction_info
            .insert(id, (res.source, filter, rider));
        state.players[you.get() as usize].mana_pool.add_restricted(
            baylee_core::mana::RestrictedMana {
                color,
                amount,
                flags: baylee_core::mana::ManaFlags::default(),
                restriction: baylee_core::mana::RestrictionId(id),
            },
        );
    } else {
        state.players[you.get() as usize]
            .mana_pool
            .add(color, amount);
    }
    state.journal.record(GameEvent::ManaProduced {
        player: you,
        color,
        amount,
        source: Some(res.source),
    });
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
        ManaSource::Fixed(color) => vec![color],
        ManaSource::Choice(colors) => colors.to_vec(),
        ManaSource::CommanderIdentity => {
            // The marker list, not the command zone: colour identity is a
            // property of the commander card wherever it is (CR 903.4), so
            // reading the zone made an Arcane Signet stop producing the
            // moment its commander was cast — exactly when it matters.
            let mut colors = ColorSet::EMPTY;
            for c in state
                .commanders
                .get(you.get() as usize)
                .into_iter()
                .flatten()
            {
                if let Some(obj) = state.object(c.object) {
                    colors = colors.union(obj.characteristics().color_identity);
                }
            }
            let options = colored(colors);
            if options.is_empty() {
                // No commander at all (a non-commander game): the ability
                // still resolves, and colorless is what is left.
                vec![ManaColor::Colorless]
            } else {
                options
            }
        }
        ManaSource::LandColor { mine } => {
            // Union of what the lands of the chosen side could produce.
            let mut colors = ColorSet::EMPTY;
            let mut colorless = false;
            for id in state.zones.list(ZoneLocation::Battlefield) {
                let Some(obj) = state.object(*id) else {
                    continue;
                };
                let c = obj.characteristics();
                if !c.types.contains(baylee_core::types::TypeSet::LAND) {
                    continue;
                }
                if (obj.controller == you) != mine {
                    continue;
                }
                colors = colors.union(c.produced_colors);
                colorless |= c.produced_colorless;
                // `produced_colors` is a reading of the *card*, and a land
                // whose mana is "the chosen color" has none there to read —
                // the answer is on this object. Without this a Reflecting
                // Pool beside an Uncharted Haven saw a land that makes
                // nothing.
                if let Some(chosen) = obj.chosen_color
                    && c.produced_chosen
                {
                    colors = colors.union(color_set_of(chosen));
                }
            }
            let mut options = colored(colors);
            if colorless {
                options.push(ManaColor::Colorless);
            }
            options
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

/// One colour as a [`ColorSet`].
fn color_set_of(c: ManaColor) -> ColorSet {
    use baylee_core::color::Color;
    match c {
        ManaColor::White => ColorSet::of(Color::White),
        ManaColor::Blue => ColorSet::of(Color::Blue),
        ManaColor::Black => ColorSet::of(Color::Black),
        ManaColor::Red => ColorSet::of(Color::Red),
        ManaColor::Green => ColorSet::of(Color::Green),
        ManaColor::Colorless => ColorSet::EMPTY,
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
        id
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
            vec![ManaColor::Colorless],
            "a game with no commander still resolves the ability"
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
            vec![ManaColor::Colorless],
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
            colors_of(&state, me(), ManaSource::LandColor { mine: true }, pool),
            vec![ManaColor::White, ManaColor::Green],
            "my lands, in WUBRG order, and the Signet is not a land"
        );
        assert_eq!(
            colors_of(&state, me(), ManaSource::LandColor { mine: false }, pool),
            vec![ManaColor::Red],
            "an Exotic Orchard reads the other side of the table"
        );

        state
            .object_mut(pool)
            .expect("still there")
            .base_mut()
            .produced_colorless = true;
        assert_eq!(
            colors_of(&state, me(), ManaSource::LandColor { mine: true }, pool),
            vec![ManaColor::White, ManaColor::Green, ManaColor::Colorless],
            "a land making colorless mana is a colour this can produce, and \
             it is not in the ColorSet the other four came out of"
        );
    }

    /// `produced_colors` is a reading of the **card**, and a land whose mana
    /// is "the chosen color" has none there to read — the answer is on the
    /// object. Without this a Reflecting Pool beside an Uncharted Haven saw
    /// a land that makes nothing.
    #[test]
    fn a_land_making_the_chosen_colour_is_read_off_the_object() {
        let mut state = state();
        let pool = land(&mut state, me(), "Reflecting Pool", ColorSet::EMPTY);
        let haven = land(&mut state, me(), "Uncharted Haven", ColorSet::EMPTY);
        state
            .object_mut(haven)
            .expect("just made it")
            .base_mut()
            .produced_chosen = true;

        assert!(
            colors_of(&state, me(), ManaSource::LandColor { mine: true }, pool).is_empty(),
            "nothing has been chosen yet, so the card is right that it \
             produces nothing"
        );

        state.object_mut(haven).expect("still there").chosen_color = Some(ManaColor::Blue);
        assert_eq!(
            colors_of(&state, me(), ManaSource::LandColor { mine: true }, pool),
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
