//! What a permanent does *as it enters*, swept over every land face in the
//! pool.
//!
//! This is the behaviour half of the testing plan, and it works the way the
//! plan splits the job in two: `xtask validate` pins a card's **data** to
//! what Scryfall prints, and a sweep here pins the **engine's behaviour** to
//! that data. Neither test does both jobs, so neither is circular — the
//! `CardDef` is a fair oracle here precisely because a different program,
//! with a different failure mode, has already checked it against the
//! printing.
//!
//! Entering tapped is the cheapest place to start: `EnterModifier::Tapped`
//! is one field, playing a land costs no mana, and a land is the only
//! permanent a seat can put onto the battlefield on turn one for free.
//!
//! It has to be a real `PlayLand`, and that is the whole reason this module
//! is not four lines. `SeatSpec::starting_battlefield` seeds a permanent with
//! `move_object(.., Cause::Setup)` (`state.rs`), which is a placement rather
//! than an entry: no replacement effect looks at it. A board built that way
//! is nonetheless untapped by the time anybody can look at it, and the
//! reason is worth stating exactly, because the obvious reading is wrong.
//! Measured on a seeded Bojuka Bog: `apply_enter_modifiers` *does* reach it —
//! it is tapped after the first mulligan is kept — and then the first turn's
//! untap step untaps it (CR 502.3, journalled as `ObjectUntapped { cause:
//! TurnBased }`) before the first priority. So a test built on `Cause::Setup`
//! would have measured nothing and passed, but not because the modifier was
//! skipped. [`printed_tests`] leans on the half of that which survives: a
//! modifier that *asks* — `TappedOrPayLife`, `ChooseSubtype` — interrupts on
//! that same pass and has to be answered.
//!
//! [`printed_tests`]: super::printed_tests
//!
//! Both arms run, because only one of them is the bug anybody expects. A
//! card that says it enters tapped and does not is the obvious fault; a card
//! that says nothing and arrives tapped anyway is the one a sweep written in
//! a single direction never sees.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, play_land_face, reach_main_phase,
};
use super::*;
use baylee_cards_dsl::{CardDef, EnterModifier, FaceDef};
use baylee_core::ids::CardIndex;

/// Bojuka Bog: a tapland that also carries an enters-the-battlefield
/// trigger, so the fixture is not the easiest possible case.
fn bojuka_bog() -> CardIndex {
    card_index("04b7362d-0490-4cb0-b5d7-2a7732f659ce")
}

/// What a card's own data says about the way it arrives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arrival {
    /// Nothing in `enter_modifiers`: it comes down ready to use.
    Untapped,
    /// `EnterModifier::Tapped`, and nothing else.
    Tapped,
    /// A modifier that asks a question — of the board (`TappedUnless`), of
    /// the player (`TappedOrPayLife`), or of neither but still mid-entry
    /// (`ChooseSubtype`). Skipped here and **counted**, because a skip
    /// bucket nobody measures is where a whole pool quietly ends up.
    Asks,
}

/// What one *face* says, which is the only reading that can be right.
///
/// `apply_enter_modifiers` used to look at `faces[0]`, and Glasspool Shore —
/// a creature on the front, a tapland on the back — came down untapped and
/// made mana the turn it landed. A sweep that read the front face would have
/// agreed with the bug.
fn arrival(face: &FaceDef) -> Arrival {
    match face.enter_modifiers {
        [] => Arrival::Untapped,
        [EnterModifier::Tapped] => Arrival::Tapped,
        _ => Arrival::Asks,
    }
}

/// Every face of `def` that is a land, by index.
///
/// Not "is the front face a land": a pathway prints two of them and chooses
/// between them as it is played, and Glasspool Mimic keeps its land on the
/// back, behind a creature.
fn land_faces(def: &CardDef) -> Vec<usize> {
    def.faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.types.contains(TypeSet::LAND))
        .map(|(i, _)| i)
        .collect()
}

/// Whether `card` arrived tapped, played as land face `face` on turn one.
///
/// The board is [`testkit::play_land_face`], which is shared with the land
/// mana sweep: one builder means one answer to "was this card actually put
/// into play", and the face-aware half of it is what makes this sweep see
/// Glasspool Shore at all.
fn played_tapped(card: CardIndex, face: usize) -> Result<bool, String> {
    let (engine, land) = play_land_face(card, face)?;
    Ok(engine
        .state()
        .object(land)
        .ok_or("was played and then vanished")?
        .status
        .contains(Status::TAPPED))
}

/// The comparison, with the card's data on one side and the game on the
/// other — separate from both so the counter-test can feed it a pair that
/// does not belong together.
fn disagreement(what: &str, face: &FaceDef, tapped: bool) -> Option<String> {
    match (arrival(face), tapped) {
        (Arrival::Tapped, false) => Some(format!(
            "{what} prints EnterModifier::Tapped and arrived untapped"
        )),
        (Arrival::Untapped, true) => Some(format!(
            "{what} prints no enter modifier at all and arrived tapped"
        )),
        _ => None,
    }
}

/// What one chunk of the pool managed.
#[derive(Default)]
struct Tally {
    /// Land faces whose data says they enter tapped, and did.
    tapped: usize,
    /// Land faces whose data says nothing, and arrived ready.
    untapped: usize,
    /// Land faces carrying a modifier that asks a question.
    asks: usize,
}

impl Tally {
    fn absorb(&mut self, other: &Self) {
        self.tapped += other.tapped;
        self.untapped += other.untapped;
        self.asks += other.asks;
    }
}

/// How small each arm is allowed to get before the sweep is no longer
/// measuring anything.
///
/// These are the guard against the failure this whole tier is written
/// around: a sweep that finds nothing is indistinguishable from a sweep that
/// checks nothing. They sit under the counts a full run reports, with room
/// for a card to be re-read or a cycle to be adopted.
///
/// Measured 2026-09-10 over the whole pool: **1217 land faces**, of which
/// 243 enter tapped, 931 enter untapped and 43 ask a question. The tapped
/// count is worth reading twice — there are 242 card *files* carrying an
/// unconditional `EnterModifier::Tapped`, and the extra one is Glasspool
/// Shore, which is a tapland on the back of a creature. A sweep that had
/// read only front faces would have reported 242 and looked right.
const TAPPED_FLOOR: usize = 200;
const UNTAPPED_FLOOR: usize = 700;

fn walk(slice: &[&'static CardDef]) -> (Vec<String>, Tally) {
    let mut offenders = Vec::new();
    let mut tally = Tally::default();
    for def in slice {
        for index in land_faces(def) {
            let face = &def.faces[index];
            if arrival(face) == Arrival::Asks {
                tally.asks += 1;
                continue;
            }
            match played_tapped(def.index, index) {
                Err(why) => offenders.push(format!("{} {why}", face.name)),
                Ok(tapped) => {
                    if let Some(found) = disagreement(face.name, face, tapped) {
                        offenders.push(found);
                    } else if arrival(face) == Arrival::Tapped {
                        tally.tapped += 1;
                    } else {
                        tally.untapped += 1;
                    }
                }
            }
        }
    }
    (offenders, tally)
}

/// The sweep, cut into one chunk per core for `offer_tests::sweep`'s reason:
/// a board is built once per land face and there are more than a thousand of
/// them, and a test that stays cheap is a test that keeps running.
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
            .map(|h| h.join().expect("enter chunk"))
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

/// CR 614.1c: "as this enters" is a replacement effect, and the only place a
/// player ever reads its outcome is the tap state of the permanent in front
/// of them.
#[test]
fn every_land_in_the_pool_arrives_the_way_its_own_card_says_it_does() {
    let (offenders, tally) = sweep();
    println!(
        "{} entered tapped, {} untapped, {} skipped for asking a question",
        tally.tapped, tally.untapped, tally.asks
    );
    assert!(
        offenders.is_empty(),
        "{} land faces entered differently from what their data says: {offenders:#?}",
        offenders.len()
    );
    assert!(
        tally.tapped >= TAPPED_FLOOR,
        "only {} land faces entered tapped, under the floor of {TAPPED_FLOOR} — either \
         the pool lost a couple of hundred taplands or this sweep stopped reaching them",
        tally.tapped
    );
    assert!(
        tally.untapped >= UNTAPPED_FLOOR,
        "only {} land faces entered untapped, under the floor of {UNTAPPED_FLOOR}",
        tally.untapped
    );
}

/// The counter-test the sweep is worth nothing without: proof that the
/// comparison reads *two* things.
///
/// A checker that asked the engine what happened and then asked the engine
/// again what should have happened would pass every card in the pool while
/// checking none of them. So the same comparison is handed a pair that does
/// not belong together — a real tapland's data against a Forest's arrival,
/// and a Forest's data against the tapland's — and it has to complain both
/// times, in both directions.
#[test]
fn the_comparison_notices_when_the_card_and_the_game_are_not_the_same_card() {
    let bog = baylee_cards::by_index(bojuka_bog()).expect("Bojuka Bog is in the pool");
    let wood = baylee_cards::by_index(basic_forest()).expect("Forest is in the pool");
    let bog_face = &bog.faces[0];
    let wood_face = &wood.faces[0];
    assert_eq!(arrival(bog_face), Arrival::Tapped);
    assert_eq!(arrival(wood_face), Arrival::Untapped);

    assert!(
        played_tapped(bojuka_bog(), 0).expect("the Bog is played"),
        "the fixture this counter-test rests on stopped entering tapped"
    );
    assert!(
        !played_tapped(basic_forest(), 0).expect("the Forest is played"),
        "the fixture this counter-test rests on started entering tapped"
    );

    assert!(
        disagreement("the Bog", bog_face, false).is_some(),
        "a tapland's data against an untapped arrival passed"
    );
    assert!(
        disagreement("the Forest", wood_face, true).is_some(),
        "a Forest's data against a tapped arrival passed"
    );
    assert!(disagreement("the Bog", bog_face, true).is_none());
    assert!(disagreement("the Forest", wood_face, false).is_none());
}

// ---------------------------------------------------------------------------
// What a permanent *is* when it has entered
//
// The sweep above measures one bit a card writes about itself. This measures
// what the rest of the board writes about the card, at the one moment the
// engine used to get wrong: the priority the player is handed straight back
// after their own action (CR 117.3c), which `Engine::after_action` publishes
// without the machine ever running.

/// Mycosynth Lattice: every permanent is an artifact in addition to its other
/// types (CR 613, layer 4).
fn mycosynth_lattice() -> CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}

/// Padeem, Consul of Innovation: artifacts you control have hexproof
/// (layer 6).
fn padeem() -> CardIndex {
    card_index("0c7ba712-6a99-4d2f-9242-a2163a11f69c")
}

/// Darksteel Forge: artifacts you control have indestructible (layer 6).
fn darksteel_forge() -> CardIndex {
    card_index("9b3bec05-441f-4fdf-8b51-69fa8613fcd4")
}

/// A land played under the three of them is all three things **before
/// anybody is asked anything**, which is the half that was wrong.
///
/// The owner's report, and it is exact: with all three on the battlefield,
/// a land played from hand inherited none of artifact, hexproof or
/// indestructible. It was never the layer system, the filters or the cards —
/// this same board projects a land correctly the moment the *next* action
/// arrives, which is what made it look like a card bug and what let a test
/// written one step too late pass. `move_object` invalidates the projection
/// on the way in and always has; nobody recomputed it before the player was
/// shown the board and offered a legal list off it.
///
/// All three are asserted rather than the type alone, because the two
/// keywords are the layer-6 half and they only arrive if layer 4 has already
/// made the land an artifact for their filters to match (CR 613.1, applied
/// in `layers::recompute_with` against the in-progress projection). A land
/// that is an artifact with neither keyword would be a different fault in a
/// different place, and this says which one it is.
#[test]
fn a_land_is_under_the_boards_continuous_effects_the_moment_it_has_entered() {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(7, basic_forest())
        .hand(0, &[basic_forest()])
        .battlefield(0, &[mycosynth_lattice(), padeem(), darksteel_forge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);

    let land = in_hand(&engine, seat, basic_forest()).expect("the Forest reached the hand");
    engine
        .apply(seat, PlayerAction::PlayLand { card: land })
        .expect("a land drop on an empty first main phase");

    let landed = engine
        .state()
        .object(land)
        .expect("the Forest is on the battlefield");
    assert_eq!(landed.zone, crate::zone::Zone::Battlefield);
    let c = landed.characteristics();
    assert!(
        c.types.contains(TypeSet::ARTIFACT),
        "a land played under Mycosynth Lattice is not an artifact: {:?}",
        c.types
    );
    assert!(
        c.keywords.contains(baylee_cards_dsl::KeywordSet::HEXPROOF),
        "a land played under Padeem has no hexproof: {:?}",
        c.keywords
    );
    assert!(
        c.keywords
            .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE),
        "a land played under Darksteel Forge is not indestructible: {:?}",
        c.keywords
    );
}

/// What an action left behind is settled before the player is asked again.
///
/// Eroded Canyon prints "When this land enters, it deals 1 damage to target
/// opponent". Playing it used to hand the player `Pending::Priority` looking
/// at an empty stack, with the trigger still unread in the journal and the
/// question of who it hits unasked — because `after_action` published that
/// priority itself and set `awaiting_answer`, which is the first line
/// `run_machine` returns on. The machine did not run again until the *next*
/// action arrived, so nothing between two actions ever happened: not the
/// layer projection (which is how the Mycosynth Lattice report was found),
/// not the state-based actions CR 117.5 owes, and not this trigger.
///
/// CR 603.3b puts a triggered ability on the stack the next time a player
/// would receive priority, which is here — CR 117.3c hands it straight back
/// to whoever acted. Both rules are about the same moment, and the engine
/// was skipping the first of them.
#[test]
fn what_an_action_triggered_is_on_the_stack_before_the_player_is_asked_again() {
    let seat = PlayerId::new(0);
    // `{T}: Add {U} or {R}`, enters tapped, and deals 1 damage to a chosen
    // opponent as it does. The damage is what makes it a witness: a trigger
    // with a target cannot be mistaken for a board that merely looks settled.
    let canyon = card_index("852c6520-d148-4923-a312-05a9af821f24");
    let mut engine = Duel::new(7, basic_forest()).hand(0, &[canyon]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);

    let land = in_hand(&engine, seat, canyon).expect("the Canyon reached the hand");
    engine
        .apply(seat, PlayerAction::PlayLand { card: land })
        .expect("a land drop on an empty first main phase");

    // The trigger is asked about at once, rather than a turn late.
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!(
            "the land's own trigger was never put on the stack: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![PlayerId::new(1)],
            },
        )
        .expect("the only opponent is a legal target");

    // And *then* the priority CR 117.3c owes, with the trigger standing on
    // the stack under it — which is what makes a response land above it.
    let stack = engine.state().zones.list(crate::zone::ZoneLocation::Stack);
    assert_eq!(
        stack.len(),
        1,
        "the trigger is the one thing on the stack when priority comes back"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat),
        "priority returns to whoever played the land: {:?}",
        engine.pending()
    );
}
