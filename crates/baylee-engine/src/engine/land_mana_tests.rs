//! Every colour a permanent's card says it makes, made on a real board.
//!
//! This is the second Tier C sweep, and it exists because the disagreement it
//! looks for is one this project has already written down as its own bug
//! class: "a land the planner counts on and the engine refuses". Two programs
//! read the same printed mana ability — [`baylee_cards_dsl::simple_mana`],
//! which is what a client's mana planner and the view builder both ask, and
//! the engine's own resolution of `Effect::AddMana` — and nothing until now
//! made them answer alike more than one card at a time.
//!
//! It is run twice, over two populations with bounds of their own: every
//! land face, and every other artifact, creature or enchantment face that
//! prints a mana line or carries a mana ability — either, so that neither the
//! text nor the card can take a face out of the sweep by itself. Fellwar
//! Stone, Sol Ring, the Talismans, the Moxen, Llanowar Elves, Birds of
//! Paradise and Noble Hierarch are in the second. Planeswalkers are not:
//! their mana is a loyalty ability (CR 606), which is no `{T}`.
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
//! and fail this one, and so do Fellwar Stone and Llanowar Elves.
//!
//! Four claims, all swept over the pool at once:
//!
//! 0. **The printed text.** Every `{T}: Add …` line a face prints, and the
//!    mana its one basic land type gives it (CR 305.6), is made by some route
//!    the engine offers — read the way the planner reads it
//!    ([`baylee_cards_dsl::simple_mana`]), then pressed under claim 2. A line
//!    whose colours are a board's rather than the card's — "any color that a
//!    land an opponent controls could produce" (CR 106.7) — has no planner
//!    reading, so it is pressed instead, across an opponent whose lands are
//!    known. On a face whose every mana line this reader can parse, the other
//!    direction holds too: no offered route makes mana its text does not
//!    print. A mana line the reader cannot parse is listed, and the list is
//!    bounded above and below, as are the lines it reads and sets aside: a
//!    cost beyond the tap, a trigger (CR 603.1), an ability in quotation
//!    marks that this permanent grants to another object (CR 113.1a). Judged
//!    on `Coverage::Implemented` cards; a card that is not has its printed
//!    promises counted instead.
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
//!
//! A creature's `{T}` is offered only once it has been under its controller's
//! control continuously since their most recent turn began (CR 302.6), so a
//! creature — Dryad Arbor, a mana elf — is put on the battlefield before the
//! first turn ([`placed_face`]) rather than played or cast, and the sweep
//! checks the engine agrees it is not summoning sick before judging it. It
//! used to skip Dryad Arbor instead.

use super::testkit::{
    RegistryLookup, answer_one, basic_forest, card_index, placed_face, play_land_face,
    play_land_face_facing, walk_to_own_main,
};
use super::*;
use baylee_cards_dsl::{AbilityDef, CardDef, CostPart, Coverage, FaceDef, SimpleMana};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaColor;
use std::ops::RangeInclusive;

/// A way to get mana out of a permanent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Route {
    /// The mana ability a basic land type gives and no card prints
    /// (CR 305.6). The engine offers it through `legal.mana_abilities`.
    Intrinsic,
    /// Printed ability `index` of the played face, offered through
    /// `legal.abilities` like any other activated ability.
    Printed(usize),
    /// Granted slot `n` (`choice::granted_ability`): an ability the face does
    /// not print but something gave it — on these boards, only the face
    /// itself, as Enduring Vitality gives its creatures one.
    Granted(u32),
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
                if def.abilities_for_face(index)[i].is_intrinsic_mana_ability() {
                    let colors: Vec<ManaColor> = [
                        (land::PLAINS, ManaColor::White),
                        (land::ISLAND, ManaColor::Blue),
                        (land::SWAMP, ManaColor::Black),
                        (land::MOUNTAIN, ManaColor::Red),
                        (land::FOREST, ManaColor::Green),
                    ]
                    .into_iter()
                    .filter(|(subtype, _)| face.subtypes.contains(subtype))
                    .map(|(_, color)| color)
                    .collect();
                    return (!colors.is_empty() && face.types.contains(TypeSet::LAND))
                        .then_some(SimpleMana { colors, amount: 1 })
                        .ok_or(Unreadable::Reading);
                }
                baylee_cards_dsl::simple_mana(cost, effects).ok_or(Unreadable::Reading)
            }
            AbilityDef::ActivatedConditional { .. } => Err(Unreadable::Conditional),
            _ => Err(Unreadable::Reading),
        },
        // Not an ability of the card at all, so there is nothing of it to
        // read here.
        Route::Granted(_) => Err(Unreadable::Reading),
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
    /// `{T}: Add one mana of any color that a land an opponent controls
    /// could produce.` — Fellwar Stone and Exotic Orchard. A promise whose
    /// colours are the board's rather than the card's (CR 106.7), so it is
    /// judged against the lands [`facing`] puts across the table.
    OpponentsLands,
    /// A mana ability whose cost is more than the tap: a filter land's
    /// `{1}, {T}`, a sacrifice. Not expected to be offered to a player
    /// holding no mana, so not judged; counted.
    Costly,
    /// A triggered ability that adds mana — "When this creature dies, add
    /// {C}{C}{C}{C}", landfall (CR 603.1). Nothing to activate, so no route
    /// to press; counted.
    Triggered,
    /// Mana made by an ability in quotation marks, which this permanent
    /// grants to other objects (CR 113.1a) — Chromatic Lantern's lands, a
    /// token it creates. Another object's route; counted.
    Granted,
    /// A line that adds mana in words this reader does not parse: a rider
    /// sentence ("This land deals 1 damage to you", "Spend this mana
    /// only…", "Activate only if…"), a colour chosen as it entered, your own
    /// lands' colours, an amount a board counts. Listed by its text.
    Unparsed,
}

/// The one board-dependent sentence [`read_printed`] reads.
const OPPONENTS_LANDS: &str =
    "one mana of any color that a land an opponent controls could produce";

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
    // An ability word has no rules meaning (CR 207.2c): "Metalcraft — {T}:
    // Add …" costs the tap and nothing else.
    let body = body
        .split_once(" — ")
        .filter(|(word, _)| is_ability_word(word))
        .map_or(body, |(_, ability)| ability);
    if !adds_mana(body) {
        return None;
    }
    if body.contains('"') {
        return Some(Printed::Granted);
    }
    if ["When ", "Whenever ", "At "]
        .iter()
        .any(|word| body.starts_with(word))
    {
        return Some(Printed::Triggered);
    }
    let Some((cost, effect)) = body.split_once(": ") else {
        return Some(Printed::Unparsed);
    };
    if cost != "{T}" {
        return Some(Printed::Costly);
    }
    let Some(mana) = effect
        .strip_prefix("Add ")
        .and_then(|m| m.strip_suffix('.'))
        .filter(|m| !m.contains('.'))
    else {
        return Some(Printed::Unparsed);
    };
    if mana == OPPONENTS_LANDS {
        return Some(Printed::OpponentsLands);
    }
    Some(parse_mana(mana).map_or(Printed::Unparsed, Printed::Promise))
}

/// Whether the words before a " — " are an ability word: one to three plain
/// words, no symbol, no cost. "Choose one —" ends its line, so it never
/// reaches here with an ability after it.
fn is_ability_word(word: &str) -> bool {
    !word.is_empty()
        && word.split(' ').count() <= 3
        && word
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == ' ' || c == '\'')
}

/// Whether a line adds *mana*: some "add" whose sentence names a mana symbol
/// or the word. A Saga's "add a lore counter" and a Class's "add its
/// ability" are not mana, and reading them as unparsed mana lines would
/// stop the reverse direction being judged on a face with nothing more to
/// say.
fn adds_mana(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.match_indices("add ").any(|(at, _)| {
        let rest = &lower[at..];
        let sentence = rest.split_once('.').map_or(rest, |(first, _)| first);
        sentence.contains('{') || sentence.contains("mana")
    })
}

/// `{G}`, `{C}{C}`, `{W} or {U}`, `{W}, {U}, or {B}`, `one mana of any
/// color`, `three mana of any one color`; `None` for anything else.
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
    let any_color = |amount| SimpleMana {
        colors: vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ],
        amount,
    };
    if mana == "one mana of any color" {
        return Some(any_color(1));
    }
    for (word, amount) in [("two", 2), ("three", 3)] {
        if mana
            .strip_prefix(word)
            .is_some_and(|rest| rest == " mana of any one color")
        {
            return Some(any_color(amount));
        }
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
    /// Lines promising what the opponent's lands could produce.
    opponents_lands: Vec<&'static str>,
    /// Mana lines costing more than the tap.
    costly: usize,
    /// Triggered abilities that add mana.
    triggered: usize,
    /// Mana abilities this face grants to other objects.
    granted: usize,
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
            Some(Printed::OpponentsLands) => out.opponents_lands.push(line),
            Some(Printed::Costly) => out.costly += 1,
            Some(Printed::Triggered) => out.triggered += 1,
            Some(Printed::Granted) => out.granted += 1,
            Some(Printed::Unparsed) => out.unparsed.push(line),
        }
    }
    if let Some(color) = lone_basic_color(&def.faces[index]) {
        out.promises
            .push(("its basic land type".to_owned(), (vec![color], 1)));
    }
    Ok(out)
}

/// Whether the face prints any line [`read_printed`] calls a mana line.
fn prints_mana(def: &CardDef, index: usize) -> bool {
    baylee_cards::oracle::face(def.index, index)
        .is_some_and(|text| text.lines().any(|line| read_printed(line).is_some()))
}

/// Wastes, `{T}: Add {C}.`
const WASTES: &str = "05d24b0c-904a-46b6-b42a-96a4d91a0dd4";

/// The lands the opponent controls on every board: a Plains, an Island and
/// a Wastes.
///
/// Two colours rather than five, so that "any color that a land an opponent
/// controls could produce" and "one mana of any color" are different
/// answers; and a land making colorless mana, because colorless is a type of
/// mana and not a colour (CR 106.1a, CR 106.1b), so "any color" must not
/// make it.
fn facing() -> Vec<CardIndex> {
    let [plains, island, ..] = baylee_cards::decks::basic_lands();
    vec![
        plains.expect("the pool has a Plains"),
        island.expect("the pool has an Island"),
        card_index(WASTES),
    ]
}

/// The colours a land the opponent controls could produce (CR 106.7), read
/// off those lands' own printed text like every other promise here, with
/// colorless left out because the sentence says colour.
fn facing_colors() -> Vec<ManaColor> {
    let mut made = Vec::new();
    for card in facing() {
        let def = baylee_cards::by_index(card).expect("a facing land is in the pool");
        let printed = prints(def, 0).expect("a facing land has printed text");
        for (_, (colors, _)) in printed.promises {
            made.extend(colors);
        }
    }
    ManaColor::ALL
        .iter()
        .copied()
        .filter(|c| *c != ManaColor::Colorless && made.contains(c))
        .collect()
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
            routes.push(
                crate::choice::granted_slot(*index)
                    .map_or(Route::Printed(*index as usize), Route::Granted),
            );
        }
    }
    Ok(routes)
}

/// The action that presses `route` on `source`.
fn press(route: Route, source: ObjectId) -> PlayerAction {
    match route {
        Route::Intrinsic => PlayerAction::ActivateManaAbility { source },
        Route::Printed(i) => PlayerAction::ActivateAbility {
            source,
            ability_index: u32::try_from(i).expect("an ability index fits a u32"),
        },
        Route::Granted(n) => PlayerAction::ActivateAbility {
            source,
            ability_index: crate::choice::granted_ability(n),
        },
    }
}

/// Presses `route` on a fresh board and reports the colours it offers: the
/// options of the colour question it asks, or, where it asks none, the
/// colours it put in the pool. In [`ManaColor::ALL`]'s order.
fn asked(card: CardIndex, face: usize, route: Route) -> Result<Vec<ManaColor>, String> {
    let seat = PlayerId::new(0);
    let (mut engine, source, _) = board(card, face)?;
    engine
        .apply(seat, press(route, source))
        .map_err(|err| format!("refused its own {route:?} route: {err:?}"))?;
    let pool = &engine.state().players[0].mana_pool;
    let options = match engine.pending() {
        Pending::ChooseColor { options, .. } => options.clone(),
        _ => Vec::new(),
    };
    Ok(ManaColor::ALL
        .iter()
        .copied()
        .filter(|c| options.contains(c) || pool.available(*c) > 0)
        .collect())
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
) -> Result<(Vec<(ManaColor, u32)>, bool), String> {
    let seat = PlayerId::new(0);
    let (mut engine, land, _) = board(card, face)?;
    engine
        .apply(seat, press(route, land))
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
    made: &[(ManaColor, u32)],
) -> Option<String> {
    let expected = vec![(want, u32::from(promise.amount))];
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
    /// Of those, promises of what the opponent's lands could produce.
    opponents_lands: usize,
    /// Creature faces judged on a board where they were not summoning sick.
    creatures: usize,
    /// Printed promises made by creature faces: the count that says the
    /// CR 302.6 board is not a vacuum.
    creature_promises: usize,
    /// Faces whose every mana line was read, so that an offered route the
    /// text does not print could be judged as well.
    fully_read: usize,
    /// Faces of cards that are not `Coverage::Implemented` whose printed
    /// promises were counted rather than judged.
    not_implemented: usize,
    /// Printed mana lines costing more than the tap.
    costly_lines: usize,
    /// Printed triggered abilities that add mana.
    triggered_lines: usize,
    /// Printed mana abilities granted to other objects.
    granted_lines: usize,
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
    /// Faces offered a mana ability their own printed grant gives them.
    self_granted: usize,
    /// Faces that were still answering their own arrival when the sweep
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
        self.opponents_lands += other.opponents_lands;
        self.creatures += other.creatures;
        self.creature_promises += other.creature_promises;
        self.fully_read += other.fully_read;
        self.not_implemented += other.not_implemented;
        self.costly_lines += other.costly_lines;
        self.triggered_lines += other.triggered_lines;
        self.granted_lines += other.granted_lines;
        self.entered_tapped += other.entered_tapped;
        self.conditional += other.conditional;
        self.costly += other.costly;
        self.unreadable += other.unreadable;
        self.dual_typed += other.dual_typed;
        self.self_typed += other.self_typed;
        self.self_granted += other.self_granted;
        self.busy += other.busy;
        self.unparsed.extend(other.unparsed.iter().cloned());
    }
}

/// The board a face makes its mana on, across an opponent who controls the
/// [`facing`] lands: played as a land drop where it is a land that is one,
/// and otherwise put on the battlefield before the first turn
/// ([`placed_face`]) — a transforming card's land back (CR 712.8a, #152), a
/// land that is also a creature, and every other permanent. What a face
/// *makes* does not depend on how it got there; how it *enters* does, and
/// that is `enter_tests`' question.
///
/// A creature is the reason for the second path rather than an exception to
/// it: a land drop is this turn, and a creature's `{T}` waits until it has
/// been controlled since its controller's turn began (CR 302.6).
///
/// A question the face asks as it arrives is answered the way setup answers
/// one ([`walk_to_own_main`]: a shockland's life paid, a colour chosen, an
/// optional choice declined), so that Vesuva, facing lands it could copy, is
/// still Vesuva; anything that walk does not know is answered the way the
/// testkit answers any question. A land that arrived tapped is untapped by
/// the harness. The third value says it had to be.
fn board(card: CardIndex, face: usize) -> Result<(Engine<RegistryLookup>, ObjectId, bool), String> {
    let def = baylee_cards::by_index(card).ok_or("is not in the pool")?;
    let types = def.faces[face].types;
    let dropped = types.contains(TypeSet::LAND)
        && !types.contains(TypeSet::CREATURE)
        && def.land_faces_from_hand().any(|playable| playable == face);
    let (mut engine, land) = if dropped {
        play_land_face_facing(card, face, &facing())
    } else {
        placed_face(card, face, &facing())
    }?;
    walk_to_own_main(&mut engine, PlayerId::new(0));
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

/// Every face of `def` that is an artifact, a creature or an enchantment and
/// not a land, and that prints a mana line or carries a mana ability.
///
/// Either, so that neither side can take a face out of the sweep by itself:
/// a Llanowar Elves whose ability is gone still prints `{T}: Add {G}.`, and a
/// rock whose card grew a mana ability its text never printed still has one.
fn other_permanent_faces(def: &CardDef) -> Vec<usize> {
    def.faces
        .iter()
        .enumerate()
        .filter(|(i, f)| {
            let types = f.types;
            !types.contains(TypeSet::LAND)
                && !types.contains(TypeSet::PLANESWALKER)
                && (types.contains(TypeSet::ARTIFACT)
                    || types.contains(TypeSet::CREATURE)
                    || types.contains(TypeSet::ENCHANTMENT))
                && (prints_mana(def, *i)
                    || def.abilities_for_face(*i).iter().any(is_a_mana_ability))
        })
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
/// Returns how many printed promises were made.
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
) -> usize {
    let face = &def.faces[index];
    let name = face.name;
    if def.coverage != Coverage::Implemented {
        if !printed.promises.is_empty() || !printed.opponents_lands.is_empty() {
            tally.not_implemented += 1;
        }
        return 0;
    }
    let readings: Vec<(Route, Reading)> = seen
        .iter()
        .filter_map(|route| {
            promised(face, def, index, *route)
                .ok()
                .map(|mana| (*route, normalised(&mana)))
        })
        .collect();
    let mut made = 0;
    for (line, promise) in &printed.promises {
        if readings.iter().any(|(_, reading)| reading == promise) {
            made += 1;
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
    tally.promises += made;
    made
}

/// Claim 0 for "one mana of any color that a land an opponent controls
/// could produce": some offered mana route asks for exactly the colours the
/// [`facing`] lands could produce, and makes one of each it is asked for.
/// Returns how many such promises were made.
///
/// Pressed rather than read, because there is nothing on the card to read:
/// [`baylee_cards_dsl::simple_mana`] refuses the sentence outright, and the
/// answer lives on the other side of the table (CR 106.7). `mana_routes` is
/// every mana ability the engine offered, readable or not.
fn judge_opponents_lands(
    def: &CardDef,
    index: usize,
    printed: &Prints,
    mana_routes: &[Route],
    offenders: &mut Vec<String>,
    tally: &mut Tally,
) -> usize {
    if def.coverage != Coverage::Implemented {
        return 0;
    }
    let name = def.faces[index].name;
    let want = facing_colors();
    let mut made = 0;
    for line in &printed.opponents_lands {
        let mut heard = Vec::new();
        let kept = mana_routes.iter().any(|route| {
            let colors = asked(def.index, index, *route);
            let right = colors.as_ref().is_ok_and(|colors| *colors == want)
                && want.iter().all(|color| {
                    pressed(def.index, index, *route, *color)
                        .is_ok_and(|(pool, tapped)| pool == [(*color, 1)] && tapped)
                });
            heard.push(format!("{route:?} offers {colors:?}"));
            right
        });
        if kept {
            made += 1;
        } else {
            offenders.push(format!(
                "{name} prints \"{line}\", which across a Plains, an Island and a Wastes is \
                 one mana of {want:?}, and no offered route makes exactly that: {heard:?}"
            ));
        }
    }
    tally.opponents_lands += made;
    made
}

/// Builds the face's board and reads what it is offered there, with whether
/// it is a creature; `None` where that could not be judged, having said why.
///
/// A creature that is summoning sick on it is an offender and not a skip:
/// the board exists to be one on which CR 302.6 lets its `{T}` be offered,
/// and a board that is not has stopped measuring every creature at once.
fn stand(
    def: &CardDef,
    index: usize,
    offenders: &mut Vec<String>,
    tally: &mut Tally,
) -> Option<(Vec<Route>, bool)> {
    let name = def.faces[index].name;
    let (engine, source, arrived_tapped) = match board(def.index, index) {
        Ok(board) => board,
        // A card that is not implemented may not survive its own board:
        // Faeburrow Elder is a 0/0 without the static that counts colours.
        // Counted with the busy ones, not charged to the engine.
        Err(_) if def.coverage != Coverage::Implemented => {
            tally.busy += 1;
            return None;
        }
        Err(why) => {
            offenders.push(format!("{name} {why}"));
            return None;
        }
    };
    let Some(on_board) = engine
        .state()
        .object(source)
        .filter(|o| o.zone == crate::zone::Zone::Battlefield)
    else {
        tally.busy += 1;
        return None;
    };
    let Ok(offer) = offered(&engine, source) else {
        tally.busy += 1;
        return None;
    };
    tally.entered_tapped += usize::from(arrived_tapped);
    let creature = on_board.characteristics().types.contains(TypeSet::CREATURE);
    if creature {
        if crate::combat::summoning_sick(engine.state(), on_board) {
            offenders.push(format!(
                "{name} is summoning sick on the board built for it, so its {{T}} cannot be \
                 offered (CR 302.6)"
            ));
            return None;
        }
        tally.creatures += 1;
    }
    Some((offer, creature))
}

/// Claims 2 and 3: what each readable route makes, colour by colour, and
/// that it costs the tap.
fn press_every_colour(
    def: &CardDef,
    index: usize,
    readable: &[Route],
    offenders: &mut Vec<String>,
    tally: &mut Tally,
) {
    let face = &def.faces[index];
    let name = face.name;
    for route in readable {
        let promise = promised(face, def, index, *route).expect("readable by construction");
        for want in promise.colors.clone() {
            match pressed(def.index, index, *route, want) {
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

fn walk_face(def: &CardDef, index: usize, offenders: &mut Vec<String>, tally: &mut Tally) {
    let name = def.faces[index].name;
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
    let printed = match prints(def, index) {
        Ok(printed) => printed,
        Err(why) => {
            offenders.push(format!("{name} {why}"));
            return;
        }
    };
    tally.costly_lines += printed.costly;
    tally.triggered_lines += printed.triggered;
    tally.granted_lines += printed.granted;
    tally.unparsed.extend(
        printed
            .unparsed
            .iter()
            .map(|line| format!("{name}: {line}")),
    );
    let Some((offer, creature)) = stand(def, index, offenders, tally) else {
        return;
    };
    // Every mana route the engine offered; a manland's "become a creature"
    // belongs to nobody's mana question.
    let mana_routes: Vec<Route> = offer
        .iter()
        .copied()
        .filter(|route| match route {
            Route::Intrinsic => true,
            Route::Printed(i) => def
                .abilities_for_face(index)
                .get(*i)
                .is_some_and(is_a_mana_ability),
            Route::Granted(_) => false,
        })
        .collect();
    // A permanent that grants itself a mana ability — Enduring Vitality's
    // creatures, Great Divide Guide's Allies — is offered it in the one entry
    // per permanent the CR 305.6 route also uses (`legal.mana_abilities`),
    // with its granted slot beside it. Nothing else on these boards grants
    // anything, so that entry on a face with no basic land type is the face's
    // own printed grant; one that prints no grant is still an offender below.
    let self_granted = !readable.contains(&Route::Intrinsic)
        && printed.granted > 0
        && offer.iter().any(|route| matches!(route, Route::Granted(_)));
    tally.self_granted += usize::from(self_granted);

    // Claim 1: the offer, in both directions, narrowed to routes the sweep
    // can read.
    let seen: Vec<Route> = mana_routes
        .iter()
        .copied()
        .filter(|route| match route {
            // The CR 305.6 shortcut depends on the type line and never on the
            // player's mana, so it is judged wherever the printed card is the
            // whole of that type line. A land printing two basic types that
            // was offered it anyway is Godless Shrine tapping for white and
            // never for black, and a filter that let that through would be
            // filtering out the one bug this arm exists for — which is why
            // the exception is exactly the one shape whose type line a static
            // rewrites, and not "anything the sweep could not read".
            Route::Intrinsic => !types_itself(def, index) && !self_granted,
            // A printed ability is judged only where the sweep can price it.
            // One it cannot — a filter land's `{1}, {T}`, a static condition
            // reading an empty board — is correctly withheld from a player
            // holding no mana.
            Route::Printed(_) => !counted.iter().any(|(r, _)| r == route),
            Route::Granted(_) => false,
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

    let made = judge_printed(def, index, &printed, &seen, offenders, tally)
        + judge_opponents_lands(def, index, &printed, &mana_routes, offenders, tally);
    if creature {
        tally.creature_promises += made;
    }
    press_every_colour(def, index, &readable, offenders, tally);
}

/// Which faces of a card one sweep walks.
type Faces = fn(&CardDef) -> Vec<usize>;

fn walk(slice: &[&'static CardDef], faces: Faces) -> (Vec<String>, Tally) {
    let mut offenders = Vec::new();
    let mut tally = Tally::default();
    for def in slice {
        for index in faces(def) {
            walk_face(def, index, &mut offenders, &mut tally);
        }
    }
    (offenders, tally)
}

fn sweep(faces: Faces) -> (Vec<String>, Tally) {
    let cards: Vec<&'static CardDef> = baylee_cards::all()
        .filter(|d| !faces(d).is_empty())
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| crate::engine::testkit::spawn_named(scope, move || walk(slice, faces)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("mana sweep chunk"))
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

/// How small each arm of one sweep may get before it has stopped measuring,
/// and, for what the text reader counts, how large.
///
/// The text reader's buckets are bounded on both sides, because a misread
/// marker moves lines from one bucket to another and a floor alone passes
/// that. The bounds will move with the pool; a trip is the moment to measure
/// again, not to widen.
struct Bounds {
    colors: usize,
    routes: usize,
    promises: RangeInclusive<usize>,
    opponents_lands: RangeInclusive<usize>,
    creature_promises: RangeInclusive<usize>,
    fully_read: usize,
    arrived_tapped: usize,
    costly_lines: RangeInclusive<usize>,
    triggered_lines: RangeInclusive<usize>,
    granted_lines: RangeInclusive<usize>,
    unparsed: RangeInclusive<usize>,
    self_granted: RangeInclusive<usize>,
    busy: usize,
    not_implemented: usize,
}

/// Runs one sweep, prints what it counted, and holds it to `bounds`.
fn hold(what: &str, faces: Faces, bounds: &Bounds) {
    let (offenders, tally) = sweep(faces);
    report(what, &tally);
    assert!(
        offenders.is_empty(),
        "{what}: {} disagreements between a card, its printed text and the engine: \
         {offenders:#?}",
        offenders.len()
    );
    check(what, &tally, bounds);
}

/// What one sweep counted, printed.
fn report(what: &str, tally: &Tally) {
    println!(
        "{what}: {} printed promises made ({} of them the opponent's lands', {} by creatures \
         that were not summoning sick, of {} such creatures), {} faces read whole, {} arrived \
         tapped and were untapped; {} faces of cards not implemented; printed mana lines: {} \
         costing more than the tap, {} triggered, {} granted to other objects, {} unparsed:\n{}",
        tally.promises + tally.opponents_lands,
        tally.opponents_lands,
        tally.creature_promises,
        tally.creatures,
        tally.fully_read,
        tally.entered_tapped,
        tally.not_implemented,
        tally.costly_lines,
        tally.triggered_lines,
        tally.granted_lines,
        tally.unparsed.len(),
        tally.unparsed.join("\n"),
    );
    println!(
        "{what}: {} colours proven over {} routes. Skipped: {} abilities behind a board \
         condition, {} that cost more than the tap, {} `simple_mana` will not read, {} faces \
         printing two basic land types, {} that give themselves one, {} offered the mana \
         their own grant gives them, {} still answering their own arrival or gone",
        tally.colors,
        tally.routes,
        tally.conditional,
        tally.costly,
        tally.unreadable,
        tally.dual_typed,
        tally.self_typed,
        tally.self_granted,
        tally.busy
    );
}

/// Holds one sweep's counts to its bounds.
fn check(what: &str, tally: &Tally, bounds: &Bounds) {
    let floors = [
        ("colours proven", tally.colors, bounds.colors),
        ("routes checked", tally.routes, bounds.routes),
        ("faces read whole", tally.fully_read, bounds.fully_read),
        (
            "faces that arrived tapped",
            tally.entered_tapped,
            bounds.arrived_tapped,
        ),
    ];
    for (arm, got, floor) in floors {
        assert!(
            got >= floor,
            "{what}: {got} {arm}, under the floor of {floor}"
        );
    }
    let ranges = [
        (
            "printed promises made, the opponent's lands' aside",
            tally.promises,
            &bounds.promises,
        ),
        (
            "promises of the opponent's lands made",
            tally.opponents_lands,
            &bounds.opponents_lands,
        ),
        (
            "promises made by creatures",
            tally.creature_promises,
            &bounds.creature_promises,
        ),
        (
            "mana lines costing more than the tap",
            tally.costly_lines,
            &bounds.costly_lines,
        ),
        (
            "triggered mana lines",
            tally.triggered_lines,
            &bounds.triggered_lines,
        ),
        (
            "mana lines granted to other objects",
            tally.granted_lines,
            &bounds.granted_lines,
        ),
        (
            "mana lines not parsed",
            tally.unparsed.len(),
            &bounds.unparsed,
        ),
        (
            "faces offered their own grant",
            tally.self_granted,
            &bounds.self_granted,
        ),
    ];
    for (arm, got, range) in ranges {
        assert!(
            range.contains(&got),
            "{what}: {got} {arm}, outside {range:?}"
        );
    }
    assert!(
        tally.busy <= bounds.busy,
        "{what}: {} faces could not be judged on their board, over the ceiling of {}",
        tally.busy,
        bounds.busy
    );
    assert!(
        tally.not_implemented <= bounds.not_implemented,
        "{what}: {} faces had their printed mana counted rather than judged, over the \
         ceiling of {}",
        tally.not_implemented,
        bounds.not_implemented
    );
}

/// Measured 2026-09-29 over the whole pool, with the printed text as the
/// oracle, taplands untapped by the harness and a Plains, an Island and a
/// Wastes across the table: **801 printed promises made, 1531 colours proven
/// over 1064 routes**, Exotic Orchard's promise of the opponent's lands, Dryad
/// Arbor judged as the one creature, 784 faces read whole, 588 that arrived
/// tapped. On 2026-09-10, when the card was its own oracle and taplands were
/// skipped, it was 146 colours over 132 routes.
///
/// The reader's buckets: 153 lines costing more than the tap, 2 triggered
/// (Crumbling Vestige, Branch of Vitu-Ghazi), 7 granted to other objects,
/// 183 not parsed (a rider sentence, a colour chosen as the land entered,
/// restricted mana, two colours at once, an amount a board counts), listed
/// in the output; 251 faces of cards not `Coverage::Implemented` whose
/// promises are counted rather than judged.
const LANDS: Bounds = Bounds {
    colors: 1400,
    routes: 950,
    promises: 700..=1000,
    opponents_lands: 1..=3,
    creature_promises: 1..=5,
    fully_read: 700,
    arrived_tapped: 500,
    costly_lines: 120..=200,
    triggered_lines: 1..=8,
    granted_lines: 4..=15,
    unparsed: 150..=250,
    self_granted: 0..=3,
    busy: 3,
    not_implemented: 300,
};

/// Measured 2026-09-29 over the whole pool on the same boards: **78 printed
/// promises made, 122 colours proven over 79 routes**, Fellwar Stone's
/// promise of the opponent's lands, 35 promises made by creatures among the
/// 90 creature faces judged — the count that says the CR 302.6 board is not a
/// vacuum — and 133 faces read whole.
///
/// The reader's buckets: 60 lines costing more than the tap (the Eggs, the
/// Signets, every "Sacrifice this creature: Add …"), 13 triggered (Priest of
/// Gix, Lotus Cobra, Su-Chi), 6 granted to other objects (Chromatic Lantern,
/// Enduring Vitality), 22 not parsed (the Talismans' damage rider, a chosen
/// colour, a commander's identity, an amount a board counts, Mox Opal's
/// condition); 2 faces offered the mana their own grant gives them (Enduring
/// Vitality, Great Divide Guide), 1 that could not stand on its board
/// (Faeburrow Elder, not implemented and 0/0 without its static), 2 faces of
/// cards not implemented.
const OTHER_PERMANENTS: Bounds = Bounds {
    colors: 100,
    routes: 65,
    promises: 60..=100,
    opponents_lands: 1..=3,
    creature_promises: 28..=50,
    fully_read: 110,
    arrived_tapped: 0,
    costly_lines: 50..=80,
    triggered_lines: 10..=20,
    granted_lines: 4..=10,
    unparsed: 15..=35,
    self_granted: 1..=4,
    busy: 3,
    not_implemented: 10,
};

/// CR 605.1: a mana ability is an activated ability that produces mana, does
/// not target, and does not use the stack — so the only place a player ever
/// reads its outcome is the pool.
#[test]
fn every_land_in_the_pool_makes_the_mana_its_own_card_promises() {
    hold("lands", land_faces, &LANDS);
}

/// The same claims over every artifact, creature and enchantment face that
/// prints a mana line or carries a mana ability, each creature on a board
/// where it has been under its controller's control since their turn began
/// (CR 302.6).
#[test]
fn every_other_permanent_in_the_pool_makes_the_mana_its_card_prints() {
    hold("other permanents", other_permanent_faces, &OTHER_PERMANENTS);
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
        Some(Printed::OpponentsLands) => Some(Err("opponents' lands")),
        Some(Printed::Costly) => Some(Err("costly")),
        Some(Printed::Triggered) => Some(Err("triggered")),
        Some(Printed::Granted) => Some(Err("granted")),
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

    assert_eq!(
        read("{T}: Add three mana of any one color."),
        Some(Ok((vec![White, Blue, Black, Red, Green], 3)))
    );
    assert_eq!(
        read("{T}: Add one mana of any color that a land an opponent controls could produce."),
        Some(Err("opponents' lands"))
    );
    assert_eq!(
        read("{T}: Add one mana of any type that a land you control could produce."),
        Some(Err("unparsed")),
        "your own lands, and a type rather than a colour"
    );
    assert_eq!(
        read(
            "Metalcraft — {T}: Add one mana of any color. Activate only if you control \
             three or more artifacts."
        ),
        Some(Err("unparsed")),
        "an ability word is not a cost (CR 207.2c); the condition after it is a rider"
    );
    assert_eq!(
        read("Lands you control have \"{T}: Add one mana of any color.\""),
        Some(Err("granted")),
        "the quoted ability is the lands', not this permanent's"
    );
    assert_eq!(
        read("When this creature dies, add {C}{C}{C}{C}."),
        Some(Err("triggered"))
    );
    assert_eq!(
        read("Landfall — Whenever a land you control enters, add one mana of any color."),
        Some(Err("triggered"))
    );
    assert_eq!(
        read("(As this Saga enters and after your draw step, add a lore counter.)"),
        None,
        "a lore counter is not mana"
    );
    assert_eq!(
        read("(Gain the next level as a sorcery to add its ability.)"),
        None
    );
}

/// The lands across the table on every board, and what the sweep expects
/// "any color that a land an opponent controls could produce" to be there.
///
/// The Wastes is read as making colorless mana and then left out, because
/// colorless is a type of mana and not a colour (CR 106.1a, CR 106.1b): an
/// expectation that kept it would ask Fellwar Stone for mana it may not make,
/// and one whose Wastes was never read at all would leave out nothing.
#[test]
fn across_the_table_are_two_colours_and_a_land_that_makes_none() {
    assert!(
        facing().contains(&card_index(WASTES)),
        "a table with no land making colorless mana cannot tell a colour from a type"
    );
    let wastes = baylee_cards::by_index(card_index(WASTES)).expect("Wastes is in the pool");
    assert_eq!(
        prints(wastes, 0).expect("Wastes has text").promises,
        vec![("{T}: Add {C}.".to_owned(), (vec![ManaColor::Colorless], 1))]
    );
    assert_eq!(facing_colors(), vec![ManaColor::White, ManaColor::Blue]);
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
