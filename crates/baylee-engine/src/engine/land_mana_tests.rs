//! Every colour a land's card says it makes, made on a real board.
//!
//! This is the second Tier C sweep, and it exists because the disagreement it
//! looks for is one this project has already written down as its own bug
//! class: "a land the planner counts on and the engine refuses". Two programs
//! read the same printed mana ability — [`baylee_cards_dsl::simple_mana`],
//! which is what a client's mana planner and the view builder both ask, and
//! the engine's own resolution of `Effect::AddMana` — and nothing until now
//! made them answer alike more than one card at a time.
//!
//! The oracle is the printed Oracle text, read line by line out of the
//! compiled table `baylee_cards::oracle` (which `xtask validate` holds to what
//! Scryfall prints), and the printed type line. It used to be the card
//! definition itself, and that was a circle: a land whose mana ability was
//! taken away promised nothing and was offered nothing, and the sweep agreed
//! with it. The mutation switch (`BAYLEE_MUTATE`, L5 in
//! `docs/verification-hooks.md`) reported 328 lands surviving the loss of
//! their mana; Karakas, Mishra's Factory and Dwarven Ruins, each with its
//! `{T}: Add …` replaced by `AbilityDef::Unimplemented`, pass the old sweep
//! and fail this one.
//!
//! Four claims, all swept over the pool at once:
//!
//! 0. **The printed text.** Every `{T}: Add …` line a face prints, and the
//!    mana its one basic land type gives it (CR 305.6), is made by some route
//!    the engine offers — read the way the planner reads it
//!    ([`baylee_cards_dsl::simple_mana`]), then pressed under claim 2. On a
//!    face whose every mana line this reader can parse, the other direction
//!    holds too: no offered route makes mana its text does not print. A mana
//!    line the reader cannot parse is listed, and the list is bounded above
//!    and below. Judged on `Coverage::Implemented` cards; a card that is not
//!    has its printed promises counted instead.
//! 1. **The offer.** A land is offered exactly the mana routes its card has —
//!    the CR 305.6 shortcut for a lone basic land type, plus each printed
//!    `{T}: Add …` — and no others. The second direction is the one worth
//!    having: an offer the card does not print is a land that taps for
//!    something nobody can read off it.
//! 2. **The mana.** Pressing a route puts exactly what the card promises in
//!    the pool, colour by colour. A land that may make one of several colours
//!    is asked for each of them in turn, on its own board, because Godless
//!    Shrine once tapped for white and never for black.
//! 3. **The cost.** A route that costs `{T}` leaves the land tapped. A cost
//!    that is offered and then not collected is free mana.
//!
//! A land that asks a question as it arrives is answered the way the testkit
//! answers any question, and a land that arrives tapped is untapped by the
//! harness: what a land makes does not depend on how it entered. The sweep
//! used to skip every face with an enter modifier, which is how Dwarven
//! Ruins, Ebon Stronghold and Havenwood Battleground were among the mutants
//! above; 588 faces arrive tapped on this board today.

use super::testkit::{
    RegistryLookup, answer_one, basic_forest, card_index, play_land_face, turned_land_face,
};
use super::*;
use baylee_cards_dsl::{AbilityDef, CardDef, CostPart, Coverage, FaceDef, SimpleMana};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaColor;

/// A way to get mana out of a permanent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Route {
    /// The mana ability a basic land type gives and no card prints
    /// (CR 305.6). The engine offers it through `legal.mana_abilities`.
    Intrinsic,
    /// Printed ability `index` of the played face, offered through
    /// `legal.abilities` like any other activated ability.
    Printed(usize),
}

/// The one basic land type this face prints, if it prints exactly one.
///
/// Exactly one, because CR 305.6 gives a land one mana ability *per* basic
/// type and the engine's shortcut cannot ask which — a dual is left to the
/// `AddManaChoice` ability printed on its card, which does ask. The sweep
/// therefore expects a two-type land to be offered no intrinsic route at all,
/// which is the assertion that would have caught Godless Shrine tapping only
/// for white.
fn lone_basic_color(face: &FaceDef) -> Option<ManaColor> {
    let mut only = None;
    for (subtype, color) in [
        (land::PLAINS, ManaColor::White),
        (land::ISLAND, ManaColor::Blue),
        (land::SWAMP, ManaColor::Black),
        (land::MOUNTAIN, ManaColor::Red),
        (land::FOREST, ManaColor::Green),
    ] {
        if face.subtypes.contains(&subtype) {
            if only.is_some() {
                return None;
            }
            only = Some(color);
        }
    }
    only
}

/// Why a printed mana ability is not one this sweep can hold the engine to.
///
/// Every one of these is *counted*: a skip bucket nobody measures is where a
/// whole pool quietly ends up. They are also the reason the offer check below
/// runs on readable routes only — an ability the sweep cannot price is one it
/// cannot expect to be offered either, and a filter land's `{1}, {T}` is
/// genuinely not offered to a player with no mana.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Unreadable {
    /// `ActivatedConditional`: whether it is even on depends on a board this
    /// sweep deliberately keeps empty (Bleachbone Verge wants a Plains or a
    /// Swamp beside it).
    Conditional,
    /// A cost that is not exactly `{T}` — a sacrifice, a life payment, or
    /// mana, which is every filter land and every "{1}, {T}: Add one mana of
    /// any color". Paying a cost to make mana is a different test.
    Cost,
    /// [`baylee_cards_dsl::simple_mana`] refused it: restricted mana, a
    /// commander's identity, another land's colours, an amount that is not a
    /// number, or a second effect beside the mana.
    Reading,
    /// The CR 305.6 shortcut on a face with no basic land type, or with two
    /// of them — a dual is left to the `AddManaChoice` ability printed on its
    /// card, which can ask which colour.
    NoLoneBasicType,
    /// The CR 305.6 shortcut on a land whose own static ability gives it a
    /// basic land type. Urborg, Tomb of Yawgmoth and Yavimaya, Cradle of
    /// Growth are the two in the pool — "each land is a Swamp / a Forest in
    /// addition to its other land types", and each of them is a land — so the
    /// shortcut answers about the *projected* type line and the printed card
    /// cannot say what it will find there.
    ///
    /// It arrived as a finding rather than as a thought: the two of them were
    /// offered no intrinsic route at all until `Engine::apply` began settling
    /// the board before publishing a pending, because the land's own static
    /// was registered but nothing had projected it yet. The sweep was right
    /// about the engine and the engine was wrong, which is the direction a
    /// skip bucket has to be read carefully in.
    SelfTyped,
}

/// What the card says pressing `route` does, or why the sweep cannot say.
fn promised(
    face: &FaceDef,
    def: &CardDef,
    index: usize,
    route: Route,
) -> Result<SimpleMana, Unreadable> {
    match route {
        Route::Intrinsic if types_itself(def, index) => Err(Unreadable::SelfTyped),
        Route::Intrinsic => lone_basic_color(face)
            .map(|color| SimpleMana {
                colors: vec![color],
                amount: 1,
            })
            .ok_or(Unreadable::NoLoneBasicType),
        Route::Printed(i) => match &def.abilities_for_face(index)[i] {
            AbilityDef::Activated { cost, effects, .. } => {
                if cost.parts != [CostPart::TapSelf] {
                    return Err(Unreadable::Cost);
                }
                baylee_cards_dsl::simple_mana(cost, effects).ok_or(Unreadable::Reading)
            }
            AbilityDef::ActivatedConditional { .. } => Err(Unreadable::Conditional),
            _ => Err(Unreadable::Reading),
        },
    }
}

/// A reading with the order a card happens to list its colours in taken out:
/// the colours in [`ManaColor::ALL`]'s order, and the amount.
type Reading = (Vec<ManaColor>, u8);

fn normalised(mana: &SimpleMana) -> Reading {
    let colors = ManaColor::ALL
        .iter()
        .copied()
        .filter(|c| mana.colors.contains(c))
        .collect();
    (colors, mana.amount)
}

/// What one printed line of Oracle text says about making mana.
#[derive(Debug)]
enum Printed {
    /// Exactly `{T}: Add …` and nothing else on the line, read.
    Promise(SimpleMana),
    /// A mana ability whose cost is more than the tap: a filter land's
    /// `{1}, {T}`, a sacrifice. Not expected to be offered to a player
    /// holding no mana, so not judged; counted.
    Costly,
    /// A line that adds mana in words this reader does not parse: a rider
    /// sentence ("This land deals 1 damage to you", "Spend this mana
    /// only…"), a colour chosen as it entered, another land's colours, an
    /// amount a board counts. Listed by its text.
    Unparsed,
}

/// Reads one printed line; `None` for a line that adds no mana.
///
/// A line and not a sentence, because one printed line is one ability: a
/// painland's `{T}: Add {R} or {G}. This land deals 1 damage to you.` is one
/// ability that does two things, and its first sentence alone would promise
/// a clean choice the card does not make.
fn read_printed(line: &str) -> Option<Printed> {
    // Reminder text stands in parentheses (CR 207.2a): on a line of its own
    // for a basic land type, "({T}: Add {B} or {R}.)", and after the ability
    // it summarises, "{T}: Add {C}. ({C} represents colorless mana.)".
    let body = line
        .strip_prefix('(')
        .and_then(|l| l.strip_suffix(')'))
        .unwrap_or(line);
    let body = body
        .strip_suffix(')')
        .and_then(|l| l.rsplit_once(" ("))
        .map_or(body, |(ability, _)| ability);
    if !body.to_ascii_lowercase().contains("add ") {
        return None;
    }
    let Some((cost, effect)) = body.split_once(": ") else {
        return Some(Printed::Unparsed);
    };
    if cost != "{T}" {
        return Some(Printed::Costly);
    }
    let promise = effect
        .strip_prefix("Add ")
        .and_then(|m| m.strip_suffix('.'))
        .filter(|m| !m.contains('.'))
        .and_then(parse_mana);
    Some(promise.map_or(Printed::Unparsed, Printed::Promise))
}

/// `{G}`, `{C}{C}`, `{W} or {U}`, `{W}, {U}, or {B}`, `one mana of any
/// color`; `None` for anything else.
fn parse_mana(mana: &str) -> Option<SimpleMana> {
    fn letter(symbol: &str) -> Option<ManaColor> {
        Some(match symbol {
            "W" => ManaColor::White,
            "U" => ManaColor::Blue,
            "B" => ManaColor::Black,
            "R" => ManaColor::Red,
            "G" => ManaColor::Green,
            "C" => ManaColor::Colorless,
            _ => return None,
        })
    }
    if mana == "one mana of any color" {
        return Some(SimpleMana {
            colors: vec![
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ],
            amount: 1,
        });
    }
    if mana.contains(" or ") {
        let colors = mana
            .replace(", or ", ", ")
            .replace(" or ", ", ")
            .split(", ")
            .map(|one| letter(one.strip_prefix('{')?.strip_suffix('}')?))
            .collect::<Option<Vec<_>>>()?;
        return Some(SimpleMana { colors, amount: 1 });
    }
    let symbols = mana
        .strip_prefix('{')?
        .strip_suffix('}')?
        .split("}{")
        .map(letter)
        .collect::<Option<Vec<_>>>()?;
    let first = *symbols.first()?;
    let amount = u8::try_from(symbols.len()).ok()?;
    symbols.iter().all(|s| *s == first).then(|| SimpleMana {
        colors: vec![first],
        amount,
    })
}

/// What a face's printed text promises, read without the card definition.
#[derive(Default)]
struct Prints {
    /// Each promise, with the printed words it was read from.
    promises: Vec<(String, Reading)>,
    /// Mana lines costing more than the tap.
    costly: usize,
    /// Mana lines this reader could not parse.
    unparsed: Vec<&'static str>,
}

/// Every `{T}: Add …` line of the face's Oracle text, and the mana its lone
/// basic land type gives it (CR 305.6): the type line is printed too, and
/// `validate` holds it to Scryfall like the text.
fn prints(def: &CardDef, index: usize) -> Result<Prints, String> {
    let text = baylee_cards::oracle::face(def.index, index)
        .ok_or("has no Oracle text for this face in the compiled table")?;
    let mut out = Prints::default();
    for line in text.lines() {
        match read_printed(line) {
            None => {}
            Some(Printed::Promise(mana)) => {
                out.promises.push((line.to_owned(), normalised(&mana)));
            }
            Some(Printed::Costly) => out.costly += 1,
            Some(Printed::Unparsed) => out.unparsed.push(line),
        }
    }
    if let Some(color) = lone_basic_color(&def.faces[index]) {
        out.promises
            .push(("its basic land type".to_owned(), (vec![color], 1)));
    }
    Ok(out)
}

/// Whether ability `i` of this face claims to be a mana ability at all
/// (CR 605.1), whatever the sweep can make of it.
fn is_a_mana_ability(ability: &AbilityDef) -> bool {
    matches!(
        ability,
        AbilityDef::Activated {
            mana_ability: true,
            ..
        } | AbilityDef::ActivatedConditional {
            mana_ability: true,
            ..
        }
    )
}

/// Every route the engine is offering `land` right now.
fn offered(engine: &Engine<RegistryLookup>, land: ObjectId) -> Result<Vec<Route>, String> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        return Err(format!(
            "is still answering its own arrival: {:?}",
            engine.pending()
        ));
    };
    let mut routes = Vec::new();
    if legal.mana_abilities.contains(&land) {
        routes.push(Route::Intrinsic);
    }
    for (source, index) in &legal.abilities {
        if *source == land {
            routes.push(Route::Printed(*index as usize));
        }
    }
    Ok(routes)
}

/// Presses `route` on a freshly played `face`, answering any colour question
/// with `want`, and reports the pool it left behind.
///
/// The pool is read as a plain list so the comparison below can be about what
/// a card promises rather than about a `ManaPool`'s shape.
fn pressed(
    card: CardIndex,
    face: usize,
    route: Route,
    want: ManaColor,
) -> Result<(Vec<(ManaColor, u16)>, bool), String> {
    let seat = PlayerId::new(0);
    let (mut engine, land, _) = land_board(card, face)?;
    let action = match route {
        Route::Intrinsic => PlayerAction::ActivateManaAbility { source: land },
        Route::Printed(i) => PlayerAction::ActivateAbility {
            source: land,
            ability_index: u32::try_from(i).expect("an ability index fits a u32"),
        },
    };
    engine
        .apply(seat, action)
        .map_err(|err| format!("refused its own {route:?} route: {err:?}"))?;
    // "Add one mana of any of these colours" stops and asks. Answering with
    // the colour under test is the whole point of pressing the route once per
    // colour rather than once per land.
    while let Pending::ChooseColor { player, options } = engine.pending().clone() {
        let color = if options.contains(&want) {
            want
        } else {
            return Err(format!(
                "asked for a colour and did not offer {want:?}, only {options:?}"
            ));
        };
        engine
            .apply(player, PlayerAction::ChooseColor(color))
            .map_err(|err| format!("refused {color:?} from its own list: {err:?}"))?;
    }
    let pool = &engine.state().players[0].mana_pool;
    let made = ManaColor::ALL
        .iter()
        .filter(|c| pool.available(**c) > 0)
        .map(|c| (*c, pool.available(*c)))
        .collect();
    let tapped = engine
        .state()
        .object(land)
        .ok_or("vanished as it made mana")?
        .status
        .contains(Status::TAPPED);
    Ok((made, tapped))
}

/// The comparison, separate from both sides so the counter-test can hand it a
/// pair that does not belong together.
fn disagreement(
    what: &str,
    promise: &SimpleMana,
    want: ManaColor,
    made: &[(ManaColor, u16)],
) -> Option<String> {
    let expected = vec![(want, u16::from(promise.amount))];
    if made == expected {
        None
    } else {
        Some(format!(
            "{what} says {} {want:?} and made {made:?}",
            promise.amount
        ))
    }
}

/// What one chunk of the pool managed.
#[derive(Default)]
struct Tally {
    /// (face, route, colour) triples proven: the card promised it and the
    /// board produced it.
    colors: usize,
    /// Routes whose offer was checked in both directions.
    routes: usize,
    /// Printed promises some offered route was found to make.
    promises: usize,
    /// Faces whose every mana line was read, so that an offered route the
    /// text does not print could be judged as well.
    fully_read: usize,
    /// Faces of cards that are not `Coverage::Implemented` whose printed
    /// promises were counted rather than judged.
    not_implemented: usize,
    /// Printed mana lines costing more than the tap.
    costly_lines: usize,
    /// Land faces that arrived tapped and were untapped by the harness.
    entered_tapped: usize,
    /// Mana abilities behind a board condition this sweep keeps empty.
    conditional: usize,
    /// Mana abilities whose cost is more than the tap.
    costly: usize,
    /// Mana abilities `simple_mana` refuses to read.
    unreadable: usize,
    /// Faces printing two basic land types, which is the one shape the
    /// CR 305.6 shortcut is *right* to withhold.
    dual_typed: usize,
    /// Faces whose own static ability writes their type line, so the
    /// CR 305.6 shortcut is answering about a board.
    self_typed: usize,
    /// Land faces that are also creatures, and so summoning sick (CR 302.6).
    also_a_creature: usize,
    /// Land faces that were still answering their own arrival when the sweep
    /// wanted priority, or that answered it by leaving the battlefield.
    busy: usize,
    /// `"<name>: <line>"` for every printed mana line the reader could not
    /// parse.
    unparsed: Vec<String>,
}

impl Tally {
    fn absorb(&mut self, other: &Self) {
        self.colors += other.colors;
        self.routes += other.routes;
        self.promises += other.promises;
        self.fully_read += other.fully_read;
        self.not_implemented += other.not_implemented;
        self.costly_lines += other.costly_lines;
        self.entered_tapped += other.entered_tapped;
        self.conditional += other.conditional;
        self.costly += other.costly;
        self.unreadable += other.unreadable;
        self.dual_typed += other.dual_typed;
        self.self_typed += other.self_typed;
        self.also_a_creature += other.also_a_creature;
        self.busy += other.busy;
        self.unparsed.extend(other.unparsed.iter().cloned());
    }
}

/// The board a land face makes its mana on: played as a land drop where it
/// is one, and turned over on the battlefield where it is a transforming
/// card's back (CR 712.8a, #152). What a face *makes* does not depend on how
/// it got there; how it *enters* does, and that is `enter_tests`' question.
///
/// So a question the land asks as it arrives (a shockland's life, a colour to
/// choose) is answered the way the testkit answers any question, and a land
/// that arrived tapped is untapped by the harness. The third value says it
/// had to be.
fn land_board(
    card: CardIndex,
    face: usize,
) -> Result<(Engine<RegistryLookup>, ObjectId, bool), String> {
    let def = baylee_cards::by_index(card).ok_or("is not in the pool")?;
    let (mut engine, land) = if def.land_faces_from_hand().any(|playable| playable == face) {
        play_land_face(card, face)
    } else {
        turned_land_face(card, face)
    }?;
    for _ in 0..8 {
        if matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        let Ok((player, action)) = answer_one(&engine) else {
            break;
        };
        if engine.apply(player, action).is_err() {
            break;
        }
    }
    let arrived_tapped = matches!(engine.pending(), Pending::Priority { .. })
        && engine
            .state()
            .object(land)
            .is_some_and(|o| o.status.contains(Status::TAPPED));
    if arrived_tapped {
        engine
            .dev_state_mut(PlayerId::new(0))
            .ok_or("the harness was refused the board")?
            .set_tapped(land, false);
        engine.refresh_offer();
    }
    Ok((engine, land, arrived_tapped))
}

/// Every face of `def` that is a land, by index.
fn land_faces(def: &CardDef) -> Vec<usize> {
    def.faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.types.contains(TypeSet::LAND))
        .map(|(i, _)| i)
        .collect()
}

/// Every mana route this face prints, split into the ones the sweep can hold
/// the engine to and the ones it can only count.
fn routes(def: &CardDef, index: usize) -> (Vec<Route>, Vec<(Route, Unreadable)>) {
    let face = &def.faces[index];
    let mut readable = Vec::new();
    let mut counted = Vec::new();
    let mut sort = |route: Route| match promised(face, def, index, route) {
        Ok(_) => readable.push(route),
        Err(why) => counted.push((route, why)),
    };
    sort(Route::Intrinsic);
    for (i, ability) in def.abilities_for_face(index).iter().enumerate() {
        if is_a_mana_ability(ability) {
            sort(Route::Printed(i));
        }
    }
    // A face printing no basic land type at all has no intrinsic route to
    // count — only a face printing *two* does, and that is the one shape the
    // CR 305.6 shortcut is right to withhold. A land that types itself is the
    // exception and is kept: it prints no basic type either, and it is
    // nonetheless offered the route on a real board.
    if face.subtypes.iter().all(|s| !is_a_basic_type(*s)) {
        counted.retain(|(route, why)| *route != Route::Intrinsic || *why == Unreadable::SelfTyped);
    }
    (readable, counted)
}

/// Whether this face's own static abilities hand it a basic land type.
///
/// Read off the card rather than off a board, like everything else the oracle
/// half of this sweep reads: a static that adds one of the five subtypes in
/// layer 4 (CR 613, and CR 305.7 for what a land then makes) is a land whose
/// projected type line is not its printed one. The filter is not consulted,
/// which makes this deliberately wide — a card adding Swamp to *other* lands
/// only would be counted here too. Widening a counted skip is the safe
/// direction and the count is printed; narrowing it by reading a filter this
/// sweep has no board for would not be.
fn types_itself(def: &CardDef, index: usize) -> bool {
    def.abilities_for_face(index).iter().any(|ability| {
        matches!(
            ability,
            AbilityDef::Static(baylee_cards_dsl::StaticAbility {
                layer: baylee_cards_dsl::Layer::Type,
                modifier: baylee_cards_dsl::Modifier::AddSubtype(s),
                ..
            }) if is_a_basic_type(*s)
        )
    })
}

/// Whether a subtype is one of the five that make mana by themselves.
fn is_a_basic_type(subtype: baylee_core::ids::SubtypeId) -> bool {
    [
        land::PLAINS,
        land::ISLAND,
        land::SWAMP,
        land::MOUNTAIN,
        land::FOREST,
    ]
    .contains(&subtype)
}

/// Claim 0: the printed text, against what the offered routes read as.
///
/// Sets rather than a pairing: a lone basic land type is offered twice, as
/// the CR 305.6 route and as its printed twin (observed-faults #57), and one
/// reminder line is what both of them make.
fn judge_printed(
    def: &CardDef,
    index: usize,
    printed: &Prints,
    seen: &[Route],
    offenders: &mut Vec<String>,
    tally: &mut Tally,
) {
    let face = &def.faces[index];
    let name = face.name;
    if def.coverage == Coverage::Implemented {
        let readings: Vec<(Route, Reading)> = seen
            .iter()
            .filter_map(|route| {
                promised(face, def, index, *route)
                    .ok()
                    .map(|mana| (*route, normalised(&mana)))
            })
            .collect();
        for (line, promise) in &printed.promises {
            if readings.iter().any(|(_, reading)| reading == promise) {
                tally.promises += 1;
            } else {
                offenders.push(format!(
                    "{name} prints \"{line}\" and is offered no route that makes {promise:?}"
                ));
            }
        }
        if printed.unparsed.is_empty() {
            tally.fully_read += 1;
            for (route, reading) in &readings {
                if !printed
                    .promises
                    .iter()
                    .any(|(_, promise)| promise == reading)
                {
                    offenders.push(format!(
                        "{name} is offered {route:?} for {reading:?}, which it does not print"
                    ));
                }
            }
        }
    } else if !printed.promises.is_empty() {
        tally.not_implemented += 1;
    }
}

fn walk_face(def: &CardDef, index: usize, offenders: &mut Vec<String>, tally: &mut Tally) {
    let face = &def.faces[index];
    // A land that is also a creature is summoning sick on the turn it lands
    // (CR 302.6), so its `{T}` is not available and the engine is right to
    // withhold it. Dryad Arbor is the whole of this arm, and it is a question
    // for a sickness test rather than for a mana one.
    if face.types.contains(TypeSet::CREATURE) {
        tally.also_a_creature += 1;
        return;
    }
    let name = face.name;
    let (readable, counted) = routes(def, index);
    for (_, why) in &counted {
        match why {
            Unreadable::Conditional => tally.conditional += 1,
            Unreadable::Cost => tally.costly += 1,
            Unreadable::Reading => tally.unreadable += 1,
            Unreadable::NoLoneBasicType => tally.dual_typed += 1,
            Unreadable::SelfTyped => tally.self_typed += 1,
        }
    }

    // Claim 1: the offer, in both directions. The engine's side is narrowed
    // to mana abilities the sweep can read — a manland's "become a creature"
    // belongs to nobody's mana question, and an ability with a cost this
    // player cannot pay is correctly not offered.
    let printed = match prints(def, index) {
        Ok(printed) => printed,
        Err(why) => {
            offenders.push(format!("{name} {why}"));
            return;
        }
    };
    tally.costly_lines += printed.costly;
    tally.unparsed.extend(
        printed
            .unparsed
            .iter()
            .map(|line| format!("{name}: {line}")),
    );
    let (engine, land, arrived_tapped) = match land_board(def.index, index) {
        Ok(board) => board,
        Err(why) => {
            offenders.push(format!("{name} {why}"));
            return;
        }
    };
    let still_there = engine
        .state()
        .object(land)
        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield);
    let Some(seen) = offered(&engine, land).ok().filter(|_| still_there) else {
        tally.busy += 1;
        return;
    };
    tally.entered_tapped += usize::from(arrived_tapped);
    let seen: Vec<Route> = seen
        .into_iter()
        .filter(|route| match route {
            // The CR 305.6 shortcut depends on the type line and never on the
            // player's mana, so it is judged wherever the printed card is the
            // whole of that type line. A land printing two basic types that
            // was offered it anyway is Godless Shrine tapping for white and
            // never for black, and a filter that let that through would be
            // filtering out the one bug this arm exists for — which is why
            // the exception is exactly the one shape whose type line a static
            // rewrites, and not "anything the sweep could not read".
            Route::Intrinsic => !types_itself(def, index),
            // A printed ability is judged only where the sweep can price it.
            // One it cannot — a filter land's `{1}, {T}`, a static condition
            // reading an empty board — is correctly withheld from a player
            // holding no mana, and a manland's "become a creature" is not a
            // mana question at all.
            Route::Printed(i) => {
                def.abilities_for_face(index)
                    .get(*i)
                    .is_some_and(is_a_mana_ability)
                    && !counted.iter().any(|(r, _)| r == route)
            }
        })
        .collect();
    for route in &readable {
        if !seen.contains(route) {
            offenders.push(format!("{name} prints {route:?} and was not offered it"));
        }
    }
    for route in &seen {
        if !readable.contains(route) {
            offenders.push(format!(
                "{name} was offered {route:?} and prints no such thing"
            ));
        }
    }
    tally.routes += readable.len();

    judge_printed(def, index, &printed, &seen, offenders, tally);

    // Claims 2 and 3: what each route makes, and that it costs the tap.
    for route in readable {
        let promise = promised(face, def, index, route).expect("readable by construction");
        for want in promise.colors.clone() {
            match pressed(def.index, index, route, want) {
                Err(why) => offenders.push(format!("{name} {why}")),
                Ok((made, tapped)) => {
                    if let Some(found) = disagreement(name, &promise, want, &made) {
                        offenders.push(found);
                    } else if tapped {
                        tally.colors += 1;
                    } else {
                        offenders.push(format!(
                            "{name} made its {want:?} for a {{T}} it never paid"
                        ));
                    }
                }
            }
        }
    }
}

fn walk(slice: &[&'static CardDef]) -> (Vec<String>, Tally) {
    let mut offenders = Vec::new();
    let mut tally = Tally::default();
    for def in slice {
        for index in land_faces(def) {
            walk_face(def, index, &mut offenders, &mut tally);
        }
    }
    (offenders, tally)
}

fn sweep() -> (Vec<String>, Tally) {
    let cards: Vec<&'static CardDef> = baylee_cards::all()
        .filter(|d| !land_faces(d).is_empty())
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| scope.spawn(move || walk(slice)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("land mana chunk"))
            .fold(
                (Vec::new(), Tally::default()),
                |(mut all, mut total), (found, one)| {
                    all.extend(found);
                    total.absorb(&one);
                    (all, total)
                },
            )
    })
}

/// How small each arm may get before the sweep has stopped measuring, and,
/// for what the text reader counts, how large.
///
/// Measured 2026-09-29 over the whole pool, with the printed text as the
/// oracle and taplands untapped by the harness: **797 printed promises made,
/// 1529 colours proven over 1062 routes**, 778 faces whose every mana line was
/// read (so the reverse direction was judged on them), 588 faces that arrived
/// tapped. On 2026-09-10, when the card was its own oracle and taplands were
/// skipped, it was 146 colours over 132 routes.
///
/// The text reader's buckets are bounded on both sides, because a misread
/// marker moves lines from one bucket to another and a floor alone passes
/// that: 189 mana lines it does not parse (a rider sentence, a colour chosen
/// as the land entered, restricted mana, two colours at once, an amount a
/// board counts), listed in the output; 249 faces of cards not
/// `Coverage::Implemented` whose promises are counted rather than judged.
const COLOR_FLOOR: usize = 1400;
const ROUTE_FLOOR: usize = 950;
const PROMISES: std::ops::RangeInclusive<usize> = 700..=1000;
const FULLY_READ_FLOOR: usize = 700;
const ARRIVED_TAPPED_FLOOR: usize = 500;
const UNPARSED: std::ops::RangeInclusive<usize> = 150..=250;
const NOT_IMPLEMENTED_CEILING: usize = 300;

/// CR 605.1: a mana ability is an activated ability that produces mana, does
/// not target, and does not use the stack — so the only place a player ever
/// reads its outcome is the pool.
#[test]
fn every_land_in_the_pool_makes_the_mana_its_own_card_promises() {
    let (offenders, tally) = sweep();
    println!(
        "{} printed promises made, {} faces read whole, {} arrived tapped and were untapped; \
         {} faces of cards not implemented, {} printed mana lines costing more than the tap, \
         {} printed mana lines unparsed:\n{}",
        tally.promises,
        tally.fully_read,
        tally.entered_tapped,
        tally.not_implemented,
        tally.costly_lines,
        tally.unparsed.len(),
        tally.unparsed.join("\n"),
    );
    println!(
        "{} colours proven over {} routes. Skipped: \
         {} that are also creatures, {} abilities behind a board condition, {} that cost \
         more than the tap, {} `simple_mana` will not read, {} faces printing two basic \
         land types, {} that give themselves one, {} still answering their own arrival",
        tally.colors,
        tally.routes,
        tally.also_a_creature,
        tally.conditional,
        tally.costly,
        tally.unreadable,
        tally.dual_typed,
        tally.self_typed,
        tally.busy
    );
    assert!(
        offenders.is_empty(),
        "{} disagreements between a land, its printed text and the engine: {offenders:#?}",
        offenders.len()
    );
    assert!(
        tally.colors >= COLOR_FLOOR,
        "only {} colours were proven, under the floor of {COLOR_FLOOR}",
        tally.colors
    );
    assert!(
        tally.routes >= ROUTE_FLOOR,
        "only {} routes were checked, under the floor of {ROUTE_FLOOR}",
        tally.routes
    );
    assert!(
        PROMISES.contains(&tally.promises),
        "{} printed promises were made, outside {PROMISES:?}",
        tally.promises
    );
    assert!(
        tally.fully_read >= FULLY_READ_FLOOR,
        "only {} faces were read whole, under the floor of {FULLY_READ_FLOOR}",
        tally.fully_read
    );
    assert!(
        tally.entered_tapped >= ARRIVED_TAPPED_FLOOR,
        "only {} faces arrived tapped, under the floor of {ARRIVED_TAPPED_FLOOR}: \
         are the taplands still being swept?",
        tally.entered_tapped
    );
    assert!(
        UNPARSED.contains(&tally.unparsed.len()),
        "{} printed mana lines were not parsed, outside {UNPARSED:?}",
        tally.unparsed.len()
    );
    assert!(
        tally.not_implemented <= NOT_IMPLEMENTED_CEILING,
        "{} faces had their printed mana counted rather than judged, over the ceiling \
         of {NOT_IMPLEMENTED_CEILING}",
        tally.not_implemented
    );
}

/// The text reader reads the shapes it claims and refuses the rest.
///
/// Its refusals are the half that matters: a line it read as a clean promise
/// when the card prints a rider would demand a route the card never had, and a
/// line it dropped entirely would stop the other direction being judged on a
/// face with more to say.
#[test]
fn the_printed_reader_reads_the_shapes_it_claims_and_refuses_the_rest() {
    use ManaColor::{Black, Blue, Colorless, Green, Red, White};
    let read = |line: &str| match read_printed(line) {
        Some(Printed::Promise(mana)) => Some(Ok(normalised(&mana))),
        Some(Printed::Costly) => Some(Err("costly")),
        Some(Printed::Unparsed) => Some(Err("unparsed")),
        None => None,
    };
    assert_eq!(read("{T}: Add {G}."), Some(Ok((vec![Green], 1))));
    assert_eq!(
        read("({T}: Add {B} or {R}.)"),
        Some(Ok((vec![Black, Red], 1)))
    );
    assert_eq!(
        read("{T}: Add {W}, {U}, or {B}."),
        Some(Ok((vec![White, Blue, Black], 1)))
    );
    assert_eq!(read("{T}: Add {C}{C}."), Some(Ok((vec![Colorless], 2))));
    assert_eq!(
        read("{T}: Add one mana of any color."),
        Some(Ok((vec![White, Blue, Black, Red, Green], 1)))
    );
    assert_eq!(
        read("{T}: Add {C}. ({C} represents colorless mana.)"),
        Some(Ok((vec![Colorless], 1))),
        "reminder text after the ability is not part of it (CR 207.2a)"
    );

    assert_eq!(
        read("{T}: Add {R} or {G}. This land deals 1 damage to you."),
        Some(Err("unparsed")),
        "a rider sentence on the same line"
    );
    assert_eq!(
        read("{T}: Add {W}{U}."),
        Some(Err("unparsed")),
        "two colours at once"
    );
    assert_eq!(
        read("{T}: Add one mana of the chosen color."),
        Some(Err("unparsed"))
    );
    assert_eq!(read("{1}, {T}: Add {W}{U}."), Some(Err("costly")));
    assert_eq!(
        read("{T}, Sacrifice this land: Add {R}{R}."),
        Some(Err("costly"))
    );
    assert_eq!(read("This land enters tapped."), None);
    assert_eq!(
        read(
            "{4}, {T}: Create a tapped Powerstone token. (It's an artifact with \
             \"{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.\")"
        ),
        None,
        "a token's mana, told in reminder text, is not this land's"
    );
}

/// The counter-test, and the reason the comparison is its own function.
///
/// A sweep that asked the engine what happened and then asked the engine what
/// should have happened would agree with itself about all of it. So the same
/// comparison is handed pairs that do not belong together — a Forest's promise
/// against a Swamp's pool and the other way round, and the right colour in the
/// wrong amount — and it has to complain every time.
#[test]
fn the_comparison_notices_when_the_promise_and_the_pool_are_not_the_same_land() {
    let forest = basic_forest();
    let swamp = card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68");
    let (green, _) =
        pressed(forest, 0, Route::Intrinsic, ManaColor::Green).expect("a Forest taps for green");
    let (black, _) =
        pressed(swamp, 0, Route::Intrinsic, ManaColor::Black).expect("a Swamp taps for black");
    assert_eq!(green, vec![(ManaColor::Green, 1)]);
    assert_eq!(black, vec![(ManaColor::Black, 1)]);

    let one_green = SimpleMana {
        colors: vec![ManaColor::Green],
        amount: 1,
    };
    let two_green = SimpleMana {
        colors: vec![ManaColor::Green],
        amount: 2,
    };
    assert!(
        disagreement("a Forest", &one_green, ManaColor::Green, &black).is_some(),
        "a Forest's promise against a Swamp's pool passed"
    );
    assert!(
        disagreement("a Swamp", &one_green, ManaColor::Black, &green).is_some(),
        "a Swamp's colour against a Forest's pool passed"
    );
    assert!(
        disagreement("a Forest", &two_green, ManaColor::Green, &green).is_some(),
        "one mana passed for a promise of two"
    );
    assert!(disagreement("a Forest", &one_green, ManaColor::Green, &green).is_none());
}

/// What the `SelfTyped` bucket is skipping, asserted rather than counted.
///
/// A skip that nobody measures is where a whole pool ends up, and this one is
/// worse than most: it was opened *because* the two cards in it were being
/// offered nothing, and closing the sweep's mouth about them without saying
/// what the right answer is would have preserved the bug as a tally line. So
/// the other direction is nailed down here. Urborg, Tomb of Yawgmoth makes
/// every land a Swamp in addition to its other land types, Urborg is a land,
/// and CR 305.6 gives a Swamp `{T}: Add {B}` — on the turn it lands, in the
/// priority its own controller is handed straight back.
///
/// It is the acceptance of the settle in `Engine::apply` read from the other
/// end: the type line this depends on is written by a static ability that was
/// registered a moment ago and projected only because something asked.
#[test]
fn a_land_that_gives_itself_a_basic_type_taps_for_it_the_turn_it_lands() {
    let urborg = card_index("db6174d7-211d-4817-b8e4-8384594c83f9");
    let (engine, land) = play_land_face(urborg, 0).expect("Urborg is played");
    assert!(
        engine
            .state()
            .object(land)
            .expect("Urborg is on the battlefield")
            .characteristics()
            .subtypes
            .contains(land::SWAMP),
        "Urborg is not a Swamp on the turn it lands"
    );
    assert_eq!(
        offered(&engine, land).expect("Urborg holds priority"),
        vec![Route::Intrinsic],
        "Urborg was not offered the mana ability its own type line gives it"
    );
    let (made, tapped) =
        pressed(urborg, 0, Route::Intrinsic, ManaColor::Black).expect("Urborg taps for black");
    assert_eq!(made, vec![(ManaColor::Black, 1)]);
    assert!(tapped, "Urborg made mana and was left untapped");
}
