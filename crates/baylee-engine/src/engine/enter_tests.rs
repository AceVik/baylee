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

use super::synthetic::{SyntheticLookup, creature, preset};
use super::testkit::{
    Duel, RegistryLookup, basic_forest, card_index, cast_from_hand, in_graveyard, in_hand,
    keep_mulligans, on_battlefield, pass_until, play_land_face, pt, quiet_creature,
    reach_main_phase, stack_is_empty, tap_mana_except,
};
use super::*;
use baylee_cards_dsl::{Amount, CardDef, CounterKind, EnterModifier, FaceDef};
use baylee_core::ids::{CardIndex, ObjectId};

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

/// How small the population is allowed to get before the sweep is no longer
/// measuring anything.
///
/// The guard against the failure this whole tier is written around: a sweep
/// that finds nothing is indistinguishable from a sweep that checks nothing.
///
/// Measured 2026-09-10 over the whole pool: **1217 land faces**, of which
/// 243 enter tapped, 931 enter untapped and 43 ask a question. The tapped
/// count is worth reading twice — there are 242 card *files* carrying an
/// unconditional `EnterModifier::Tapped`, and the extra one is Glasspool
/// Shore, which is a tapland on the back of a creature. A sweep that had
/// read only front faces would have reported 242 and looked right.
///
/// **Those three numbers move without anything being wrong**, and only one
/// of them moves upward. Every stub is a bare `Land` with no enter modifier,
/// so it is counted untapped; finishing it moves it to whichever arm its
/// printing says, and most of what is left to finish is a tapland or a card
/// that asks. Round H finished 71 lands and the same 1217 faces read 378
/// tapped, 689 untapped and 150 asking — a floor of 700 on the untapped arm
/// went red with nothing wrong, because it was a threshold between two
/// numbers that were both moving. So the untapped arm is not held against a
/// count at all, and what the sweep is held against instead is the thing
/// that cannot drift with implementation progress: **every land face in the
/// pool is reached and classified**, which is an equality rather than a
/// floor and says strictly more than either constant did.
const TAPPED_FLOOR: usize = 200;

/// The untapped arm as a share of the faces reached — at least a quarter.
/// It exists only to say the arm has not emptied, and a share cannot be
/// crossed by finishing a card the way a count can.
const UNTAPPED_SHARE: usize = 4;

/// The pool still has its lands. Unlike the arms, this one moves only when
/// cards are added or removed, which is a fact about the pool rather than
/// about how much of it is written.
const FACES_FLOOR: usize = 1200;

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
    // Every face, not merely a lot of them. `offenders` is empty by the
    // assertion above and a face lands in exactly one of the three buckets,
    // so the three of them add back up to the population or the sweep
    // dropped something on the floor.
    let faces: usize = baylee_cards::all().map(|def| land_faces(def).len()).sum();
    let reached = tally.tapped + tally.untapped + tally.asks;
    assert_eq!(
        reached, faces,
        "the sweep classified {reached} of the pool's {faces} land faces"
    );
    assert!(
        faces >= FACES_FLOOR,
        "only {faces} land faces in the pool, under the floor of {FACES_FLOOR} — \
         either the pool lost its lands or this sweep stopped reaching them"
    );
    assert!(
        tally.tapped >= TAPPED_FLOOR,
        "only {} land faces entered tapped, under the floor of {TAPPED_FLOOR} — either \
         the pool lost a couple of hundred taplands or this sweep stopped reaching them",
        tally.tapped
    );
    assert!(
        tally.untapped * UNTAPPED_SHARE >= faces,
        "only {} of {faces} land faces entered untapped, under 1/{UNTAPPED_SHARE} \
         of them",
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

/// A made-up 0/0 that arrives under one +1/+1 counter, and the only thing
/// standing between it and a graveyard.
const HATCHLING: u32 = 1200;

static HATCHLING_ENTERS: &[EnterModifier] = &[EnterModifier::WithCounters {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];

/// Counters placed as a permanent enters are read by the state-based
/// actions that then decide whether it lives.
///
/// Two steps of `run_machine` that had never met. As-it-enters modifiers are
/// step 0b and the layer projection is refreshed at 0a, so a counter placed
/// at 0b landed *behind* the characteristics CR 704.5f reads at step 2 — and
/// a printed 0/0 that had just arrived under a +1/+1 counter was put into its
/// owner's graveyard before anybody could be asked anything. The counter was
/// there on the object the whole time; what the rule looked at was a cache
/// one step older than it (CR 613.4c makes a counter a characteristic-
/// defining input, so the two must not disagree).
///
/// Seven printed lands have gone through that seam in silence since
/// `EnterModifier::WithCounters` shipped, and none of them could have shown
/// it: a charge counter changes no characteristic any state-based action
/// reads. So the witness is a card nobody printed — a 0/0 body is the
/// smallest thing that can tell a stale projection from a fresh one, and the
/// pool has no creature that both enters with a counter and needs it.
#[test]
fn a_body_that_arrives_under_a_counter_is_alive_when_the_rules_look_at_it() {
    let card = creature(HATCHLING, "Hatchling", 0, 0, HATCHLING_ENTERS);
    let mut engine = Engine::new(&preset(11, &[HATCHLING]), SyntheticLookup::new(vec![card]))
        .expect("a two-seat board with one made-up creature on it");
    super::synthetic::keep_mulligans(&mut engine);

    let bodies = super::synthetic::permanents(&engine, HATCHLING);
    assert_eq!(
        bodies.len(),
        1,
        "the Hatchling was killed by the state-based action that read the \
         projection it had before its own counter landed (CR 704.5f)"
    );
    let body = bodies[0];
    assert_eq!(
        engine
            .state()
            .object(body)
            .map(|o| o.counters.get(CounterKind::P1P1)),
        Some(1),
        "one +1/+1 counter, placed as it entered (CR 614.1c)"
    );
    let c = engine
        .state()
        .object(body)
        .map(|o| (o.characteristics().power, o.characteristics().toughness))
        .expect("the Hatchling is on the battlefield");
    assert_eq!(
        c,
        (Some(1), Some(1)),
        "and the projection the rules read is built out of it"
    );
}

/// Walking Ballista: `{X}{X}`, a printed 0/0 whose whole body is "this
/// creature enters with X +1/+1 counters on it".
fn walking_ballista() -> CardIndex {
    card_index("4b515bb0-f275-4400-8032-3173b799ab40")
}

/// Reanimate: `{B}` sorcery, "put target creature card from a graveyard onto
/// the battlefield under your control".
fn reanimate() -> CardIndex {
    card_index("a044474a-cd72-4e9d-bd8d-a08f2de9cdc0")
}

/// A basic Swamp, for the one black mana the reanimation below costs.
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// The `+1/+1` counters sitting on one object.
///
/// Read off the object rather than off a projection, because that is the
/// half of "arrives as an X/X" a layer could not fake: `pt` below is the
/// projection, and the two together say the counters are there *and* that
/// the body is built out of them.
fn plus_ones(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(CounterKind::P1P1))
}

/// The X announced as a spell is cast reaches the permanent it becomes.
///
/// CR 107.3m is the rule with the whole shape in it: a replacement effect on
/// a permanent that refers to X uses the value of X chosen for **the spell
/// that became that object as it resolved**, and the value of X for the
/// permanent itself is 0. Both halves matter here. The first is why
/// `EnterModifier::WithCounters` carries an [`Amount`] at all and why the
/// engine reads `x_value` off the entering object; the second is the
/// [next test](a_reanimated_body_brings_none_of_the_x_it_was_cast_for).
///
/// Walking Ballista is the card the variant was widened for: a 0/0 that
/// survives its own arrival only because the counters land first — which
/// they do, because `apply_enter_modifiers` runs as step 0b of the machine
/// and state-based actions are step 2.
///
/// [`Amount`]: baylee_cards_dsl::Amount
#[test]
fn a_spell_cast_for_x_enters_with_that_many_counters() {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(11, basic_forest())
        .battlefield(
            0,
            &[
                basic_forest(),
                basic_forest(),
                basic_forest(),
                basic_forest(),
            ],
        )
        .hand(0, &[walking_ballista()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);

    // Held before it is cast, because the assertions below span three zones
    // and the object is the same one throughout — a spell that resolves as a
    // permanent keeps its id (CR 400.7 makes it a new *object* for the rules;
    // the engine rewrites the fields in place).
    let id = in_hand(&engine, seat, walking_ballista()).expect("the Ballista is in hand");

    // Four Forests tapped, then the spell — `{X}{X}` is castable for X = 0
    // whatever is floating, so what this proves is not affordability.
    cast_from_hand(&mut engine, seat, walking_ballista());

    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!(
            "a printed {{X}} has to be asked about, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(min, 0, "X may always be nothing");
    assert!(
        max >= 2,
        "four mana pays {{X}}{{X}} for X = 2, so two is inside the offered range: max = {max}"
    );
    engine
        .apply(seat, PlayerAction::ChooseNumber(2))
        .expect("X = 2 is inside the range the engine just offered");
    pass_until(&mut engine, stack_is_empty);

    // Two assertions and not one, because a printed 0/0 that is not on the
    // battlefield has two different reasons to be missing and they are two
    // different defects. The journal answers the first on its own: it keeps
    // what the counters did even if the body did not survive being looked
    // at, where the object's own `counters` are wiped as it leaves.
    assert!(
        engine.state().journal.entries().iter().any(|e| matches!(
            e.event,
            GameEvent::CounterChanged {
                object,
                kind: CounterKind::P1P1,
                old: 0,
                new: 2,
            } if object == id
        )),
        "the X announced as the spell was cast never reached its entry \
         clause (CR 107.3m)"
    );
    let ballista = on_battlefield(&engine, seat, walking_ballista()).expect(
        "the counters landed and the body still died, so the state-based \
         action read the projection it had before them (CR 704.5f against \
         CR 613.4c)",
    );
    assert_eq!(
        plus_ones(&engine, ballista),
        2,
        "X was announced as two, so two +1/+1 counters arrive with the body"
    );
    assert_eq!(
        pt(&engine, ballista),
        (2, 2),
        "a printed 0/0 under two +1/+1 counters is a 2/2"
    );
}

/// The same card brought back by an effect arrives with nothing, because
/// nobody announced an X for it.
///
/// This is the second half of CR 107.3m — "although the value of X for that
/// permanent is 0" — and it is a rule about *which* X, not about zero. The
/// object that dies keeps `x_value` on it: the reset in `move_object` fires
/// only when a permanent leaves the battlefield, and it deliberately spares
/// the spell-shaped fields so a permanent resolving off the stack still has
/// them. So a reader that simply took `x_value` would reanimate this 0/0 as
/// a 1/1 for ever, off a number chosen one zone change ago.
///
/// What stops it is one normalisation at the arrival
/// (`apply_enter_modifiers`), not a question each reader asks: an entry
/// that did not come from the stack puts the field back to 0, so every
/// reader of CR 107.3m downstream is a plain read. That is why the last
/// assertion here is about the **field** and not about the counters. The
/// counters are one reader; an enters-the-battlefield triggered ability is
/// another, and it is stacked a step later with no arrival left to ask
/// (CR 107.3g: a card anywhere but the stack has an X of 0).
#[test]
fn a_reanimated_body_brings_none_of_the_x_it_was_cast_for() {
    let (seat, foe) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, basic_forest())
        .battlefield(0, &[basic_forest(), basic_forest(), swamp()])
        .hand(0, &[walking_ballista(), reanimate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);

    // The Swamp is kept back: it is the one mana the reanimation costs, and
    // a generic `{X}{X}` would otherwise be happy to spend it.
    let held = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == swamp()))
        })
        .expect("the Swamp is on the battlefield");
    tap_mana_except(&mut engine, seat, held);
    let spell = in_hand(&engine, seat, walking_ballista()).expect("the Ballista is in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card: spell })
        .expect("two Forests pay {X}{X} for X = 1");

    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!(
            "a printed {{X}} has to be asked about, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(seat, PlayerAction::ChooseNumber(1))
        .expect("two mana pays {X}{X} for X = 1");
    pass_until(&mut engine, stack_is_empty);

    let ballista = on_battlefield(&engine, seat, walking_ballista()).expect("it arrived as a 1/1");
    assert_eq!(plus_ones(&engine, ballista), 1, "X was one");

    // Its own second ability spends that counter, and the 0/0 left behind
    // dies to the state-based action (CR 704.5f) — which is how the card
    // reaches a graveyard while still carrying `x_value = 1`.
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: ballista,
                ability_index: 1,
            },
        )
        .expect("removing its last +1/+1 counter is a cost it can pay");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![foe],
            },
        )
        .expect("the opponent is a legal target for any target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, seat, walking_ballista()).is_none(),
        "a 0/0 with no counters left is put into its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, seat, walking_ballista()).is_some(),
        "and that graveyard is its owner's"
    );

    // Now the reanimation, off the Swamp that was kept back.
    cast_from_hand(&mut engine, seat, reanimate());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "Reanimate targets a creature card in a graveyard, got {:?}",
            engine.pending()
        )
    };
    let target = in_graveyard(&engine, seat, walking_ballista()).expect("the Ballista is there");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: Vec::new(),
            },
        )
        .expect("a creature card in a graveyard is what it asks for");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, seat, walking_ballista()).is_none(),
        "nobody announced an X for a reanimation, so a printed 0/0 comes \
         back as a 0/0 and dies again — it is on the battlefield with \
         {} +1/+1 counters, off an X chosen a zone change ago",
        on_battlefield(&engine, seat, walking_ballista()).map_or(0, |id| plus_ones(&engine, id))
    );

    // And the field itself, which is what every reader of CR 107.3m now
    // takes at face value. It was 1 when the spell was cast, it rode
    // through the first death untouched, and the entry off the graveyard
    // put it back — so reading it in the graveyard after the second death
    // is reading the normalisation and nothing else. Asserting the
    // counters alone would leave a triggered ability free to announce a 1.
    let dead = in_graveyard(&engine, seat, walking_ballista()).expect("it died a second time");
    assert_eq!(
        engine.state().object(dead).map_or(u32::MAX, |o| o.x_value),
        0,
        "the X belongs to the spell that became the permanent, and no spell \
         became this one"
    );
}

// ── The reveal lands: an entry clause read against a hidden zone ──────────

/// Secluded Glen — "As this land enters, you may reveal a Faerie card from
/// your hand. If you don't, this land enters tapped."
fn secluded_glen() -> CardIndex {
    card_index("09f52275-99e6-45e0-b2db-cafe26d5fb91")
}

/// Choked Estuary — the same clause over a *basic land type*.
fn choked_estuary() -> CardIndex {
    card_index("d473b507-8c33-4118-bc10-b0a268776074")
}

/// Scryb Sprites: a 1/1 Faerie, and the cheapest one in the pool.
fn scryb_sprites() -> CardIndex {
    card_index("a1f20695-6f08-4d5c-9fba-b0018bee298e")
}

/// Whether the journal records `player` having revealed exactly `card`.
fn revealed(engine: &Engine<RegistryLookup>, player: PlayerId, card: ObjectId) -> bool {
    engine.journal().entries().iter().any(|e| {
        matches!(&e.event, GameEvent::Revealed { player: p, cards }
            if *p == player && cards.as_slice() == [card])
    })
}

/// [`EnterModifier::TappedUnlessReveal`], all four of its sentences.
///
/// This is the rule behind eighteen lands, and it is the first entry
/// modifier whose filter is read against a **hidden** zone — every
/// `TappedUnless…` sibling walks the battlefield. A sweep over the pool
/// cannot see any of this: `enter_tests`' own census puts an asking
/// modifier in the `Asks` bucket and moves on, which is exactly the bucket
/// this rule would have hidden in.
///
/// Four games, because a seat plays one land a turn and each answer is a
/// different game:
///
/// 1. **Reveal** → the land is untapped, the journal carries the reveal, and
///    the card is still in hand (CR 701.20b — showing a card does not move
///    it). All three, because dropping any one of them still passes the
///    other two: a land that arrives untapped without a journal entry was
///    shown to nobody, and a card that left the hand was discarded rather
///    than revealed.
/// 2. **Decline** → naming nothing is the legal way to say no (`min: 0`) and
///    the land comes down tapped.
/// 3. **Nothing to reveal** → no question is asked at all and the land is
///    tapped. Asking would leak that the hand holds no Faerie, and a prompt
///    whose only legal answer is "no" is not a choice.
/// 4. **A creature that is not a Faerie is not on the menu.** The filter is
///    one `HasSubtype` and no `Filter::CREATURE` beside it, which is the
///    right shape for the printed words "a Faerie card" — Magic prints
///    tribal instants and sorceries carrying a creature type, so the card
///    type would refuse a card this land accepts. *This pool prints none of
///    them* (measured: no face in 2716 cards carries a creature subtype
///    without `TypeSet::CREATURE`), so that half is a reason and not a
///    claim, and what is asserted here is the half that can be: the type
///    must not have been read as "any creature" either.
#[test]
fn a_reveal_land_reads_the_hand_and_enters_on_what_it_finds() {
    let p0 = PlayerId::new(0);

    // ── 1. Reveal: untapped, journalled, and the card stays in hand.
    {
        let mut engine = Duel::new(71, basic_forest())
            .hand(0, &[secluded_glen(), scryb_sprites()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = in_hand(&engine, p0, secluded_glen()).expect("the Glen is in hand");
        let faerie = in_hand(&engine, p0, scryb_sprites()).expect("the Sprites are in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();

        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
        } = engine.pending().clone()
        else {
            panic!(
                "a Faerie in hand makes the Glen ask, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(player, p0);
        assert_eq!(
            options,
            vec![faerie],
            "the menu is the matching cards in hand and nothing else"
        );
        assert_eq!((min, max), (0, 1), "\"you may reveal a card\"");
        assert_eq!(prompt, crate::choice::ChoicePrompt::RevealOrEnterTapped);

        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![faerie],
                },
            )
            .expect("a Faerie is what it asked for");
        pass_until(&mut engine, stack_is_empty);

        let glen = on_battlefield(&engine, p0, secluded_glen()).expect("the Glen is in play");
        assert!(
            !engine
                .state()
                .object(glen)
                .is_some_and(|o| o.status.contains(Status::TAPPED)),
            "a revealed Faerie is what keeps the Glen untapped"
        );
        assert!(
            revealed(&engine, p0, faerie),
            "the reveal is only a reveal if somebody could have seen it"
        );
        assert!(
            in_hand(&engine, p0, scryb_sprites()).is_some(),
            "CR 701.20b: revealing a card does not move it"
        );
    }

    // ── 2. Decline: naming nothing, and the land arrives tapped.
    {
        let mut engine = Duel::new(71, basic_forest())
            .hand(0, &[secluded_glen(), scryb_sprites()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = in_hand(&engine, p0, secluded_glen()).expect("the Glen is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();
        assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: Vec::new(),
                },
            )
            .expect("min is 0, so naming nothing is a legal answer");
        pass_until(&mut engine, stack_is_empty);

        let glen = on_battlefield(&engine, p0, secluded_glen()).expect("the Glen is in play");
        assert!(
            engine
                .state()
                .object(glen)
                .is_some_and(|o| o.status.contains(Status::TAPPED)),
            "declining the reveal is what the printed \"if you don't\" charges for"
        );
    }

    // ── 3. Nothing to reveal: no question, and tapped.
    {
        let mut engine = Duel::new(71, basic_forest())
            .hand(0, &[secluded_glen()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = in_hand(&engine, p0, secluded_glen()).expect("the Glen is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();
        assert!(
            !matches!(engine.pending(), Pending::ChooseCards { .. }),
            "a hand with no Faerie is not asked — the only legal answer would \
             be \"no\", and asking would say out loud what the hand is missing"
        );
        pass_until(&mut engine, stack_is_empty);

        let glen = on_battlefield(&engine, p0, secluded_glen()).expect("the Glen is in play");
        assert!(
            engine
                .state()
                .object(glen)
                .is_some_and(|o| o.status.contains(Status::TAPPED)),
            "no Faerie, no reveal, tapped"
        );
    }

    // ── 4. The subtype is the whole of the question.
    {
        let mut engine = Duel::new(71, basic_forest())
            .hand(0, &[secluded_glen(), scryb_sprites(), quiet_creature()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = in_hand(&engine, p0, secluded_glen()).expect("the Glen is in hand");
        let faerie = in_hand(&engine, p0, scryb_sprites()).expect("the Sprites are in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!("a Faerie in hand makes it ask, got {:?}", engine.pending())
        };
        assert_eq!(
            options,
            vec![faerie],
            "Llanowar Elves is a creature and is not a Faerie, so it is not \
             on the menu — the clause names a subtype, not a card type"
        );
    }
}

/// Choked Estuary reads a **basic land type**, over the same rule.
///
/// A separate game rather than a fifth arm above, because it is the other
/// half of what `Filter::HasSubtype` has to reach: `subtypes` is one sorted
/// range partitioned by kind, so a reader that had quietly bounded itself to
/// the creature partition would pass every Faerie sentence and fail here.
/// And the reveal is answered with a *basic Island*, which is the card a
/// player actually holds when this land is in their deck.
#[test]
fn a_reveal_land_can_ask_for_a_basic_land_type() {
    let p0 = PlayerId::new(0);
    let island = baylee_cards::decks::by_name("Island").expect("the pool has basic Islands");

    let mut engine = Duel::new(88, basic_forest())
        .hand(0, &[choked_estuary(), island])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = in_hand(&engine, p0, choked_estuary()).expect("the Estuary is in hand");
    let shown = in_hand(&engine, p0, island).expect("the Island is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "an Island in hand makes the Estuary ask, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![shown]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shown],
            },
        )
        .expect("an Island card is what it asked for");
    pass_until(&mut engine, stack_is_empty);

    let estuary = on_battlefield(&engine, p0, choked_estuary()).expect("the Estuary is in play");
    assert!(
        !engine
            .state()
            .object(estuary)
            .is_some_and(|o| o.status.contains(Status::TAPPED)),
        "a revealed Island keeps the Estuary untapped"
    );
    assert!(
        revealed(&engine, p0, shown),
        "and the Island was shown to the table"
    );
    assert!(
        in_hand(&engine, p0, island).is_some(),
        "a land revealed from hand is still a land drop the player may make"
    );
}
