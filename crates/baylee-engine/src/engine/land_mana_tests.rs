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
//! The oracle is the card, which is fair here for the reason the plan gives:
//! `xtask validate` has already pinned that data to what Scryfall prints, so
//! this sweep is the second of two independent links rather than a circle.
//!
//! Three claims, all swept over the pool at once:
//!
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
//! Only lands that arrive untapped are swept, and the rest are counted: a
//! land that enters tapped has no `{T}` to give on the turn it lands, and
//! walking it to the next turn would cost the sweep a turn per card for a
//! question the untapped arm already answers.

use super::testkit::{RegistryLookup, basic_forest, card_index, play_land_face};
use super::*;
use baylee_cards_dsl::{AbilityDef, CardDef, CostPart, FaceDef, SimpleMana};
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
}

/// What the card says pressing `route` does, or why the sweep cannot say.
fn promised(
    face: &FaceDef,
    def: &CardDef,
    index: usize,
    route: Route,
) -> Result<SimpleMana, Unreadable> {
    match route {
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
    let (mut engine, land) = play_land_face(card, face)?;
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
    /// Land faces skipped for arriving tapped or asking a question.
    not_untapped: usize,
    /// Mana abilities behind a board condition this sweep keeps empty.
    conditional: usize,
    /// Mana abilities whose cost is more than the tap.
    costly: usize,
    /// Mana abilities `simple_mana` refuses to read.
    unreadable: usize,
    /// Faces printing two basic land types, which is the one shape the
    /// CR 305.6 shortcut is *right* to withhold.
    dual_typed: usize,
    /// Land faces that are also creatures, and so summoning sick (CR 302.6).
    also_a_creature: usize,
    /// Land faces that were still answering their own arrival when the sweep
    /// wanted priority.
    busy: usize,
}

impl Tally {
    fn absorb(&mut self, other: &Self) {
        self.colors += other.colors;
        self.routes += other.routes;
        self.not_untapped += other.not_untapped;
        self.conditional += other.conditional;
        self.costly += other.costly;
        self.unreadable += other.unreadable;
        self.dual_typed += other.dual_typed;
        self.also_a_creature += other.also_a_creature;
        self.busy += other.busy;
    }
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
    // CR 305.6 shortcut is right to withhold.
    if face.subtypes.iter().all(|s| !is_a_basic_type(*s)) {
        counted.retain(|(route, _)| *route != Route::Intrinsic);
    }
    (readable, counted)
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

fn walk_face(def: &CardDef, index: usize, offenders: &mut Vec<String>, tally: &mut Tally) {
    let face = &def.faces[index];
    if !face.enter_modifiers.is_empty() {
        tally.not_untapped += 1;
        return;
    }
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
        }
    }

    // Claim 1: the offer, in both directions. The engine's side is narrowed
    // to mana abilities the sweep can read — a manland's "become a creature"
    // belongs to nobody's mana question, and an ability with a cost this
    // player cannot pay is correctly not offered.
    let (engine, land) = match play_land_face(def.index, index) {
        Ok(board) => board,
        Err(why) => {
            offenders.push(format!("{name} {why}"));
            return;
        }
    };
    let Ok(seen) = offered(&engine, land) else {
        tally.busy += 1;
        return;
    };
    let seen: Vec<Route> = seen
        .into_iter()
        .filter(|route| match route {
            // The CR 305.6 shortcut depends on the type line alone — never on
            // the board, never on the player's mana — so it is always judged.
            // A land printing two basic types that was offered it anyway is
            // Godless Shrine tapping for white and never for black, and a
            // filter that let that through would be filtering out the one
            // bug this arm exists for.
            Route::Intrinsic => true,
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

/// How small each arm may get before the sweep has stopped measuring.
///
/// Measured 2026-09-10 over the whole pool: **146 colours proven over 132
/// routes**, against 1217 land faces of which 286 do not arrive untapped, one
/// is also a creature (Dryad Arbor, summoning sick on the turn it lands) and
/// the rest print no mana ability the sweep can hold the engine to. That last
/// number is large and it is not a hole: 694 of the 1124 land files in the
/// pool are still `// GENERATED STUB` and have no abilities at all, and 410
/// carry a `mana_ability!` of which most enter tapped.
///
/// The skips that are about a *readable* ability come to 89 and are reported
/// one bucket at a time, because a single "skipped" number is where a whole
/// pool quietly ends up: 54 that `simple_mana` refuses on purpose (restricted
/// mana, a commander's identity, another land's colours, an amount that is
/// not a number), 25 faces printing two basic land types, 9 costing more than
/// the tap, and one behind a board condition.
const COLOR_FLOOR: usize = 120;
const ROUTE_FLOOR: usize = 110;

/// CR 605.1: a mana ability is an activated ability that produces mana, does
/// not target, and does not use the stack — so the only place a player ever
/// reads its outcome is the pool.
#[test]
fn every_land_in_the_pool_makes_the_mana_its_own_card_promises() {
    let (offenders, tally) = sweep();
    println!(
        "{} colours proven over {} routes. Skipped: {} faces that do not arrive untapped, \
         {} that are also creatures, {} abilities behind a board condition, {} that cost \
         more than the tap, {} `simple_mana` will not read, {} faces printing two basic \
         land types, {} still answering their own arrival",
        tally.colors,
        tally.routes,
        tally.not_untapped,
        tally.also_a_creature,
        tally.conditional,
        tally.costly,
        tally.unreadable,
        tally.dual_typed,
        tally.busy
    );
    assert!(
        offenders.is_empty(),
        "{} disagreements between a land and its own card: {offenders:#?}",
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
