//! The activator an activation names for its target question ends with the
//! activation.
//!
//! `Engine::activation_target_choice` holds who is activating, for a target
//! question someone else may answer, and the seats that question named. It
//! is set as an activation starts and emptied where the ability reaches the
//! stack or the activation is reversed. Two paths through
//! `start_activation` left it standing with nothing in flight: the special
//! actions it also carries (turning a permanent face up, CR 116.2b, and
//! unlocking a door, CR 116.2m) and a prepared cast named the activator and
//! returned, and a loyalty activation pushed its ability in a function of
//! its own that kept it. The field is in `Engine::snapshot_hash` and the
//! fork reads it, so every position after one of those, until the next
//! activation was pressed, hashed the activator in and was refused a fork
//! as `Continuation("activation_target_choice")`. Nothing else reads it.

use super::abilities::KEEP_STALE_TARGET_CHOICE;
use super::refusal_tests::{DECKS, Dice, house_deck, offered};
use super::testkit::*;
use super::*;
use crate::engine::{DeckKnowledge, ForkRefusal};
use baylee_core::ids::CardIndex;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);
const STALE: ForkRefusal = ForkRefusal::Continuation("activation_target_choice");

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).expect("the registry holds the card")
}

/// "+1: Create a 1/1 blue Illusion creature token. −2: Return target
/// creature to its owner's hand. −6: …" — loyalty 3. Its draw-step trigger
/// is ability 0.
fn jace() -> CardIndex {
    card_index("89a6e876-0b00-4671-9652-766fd7ef9bf2")
}

const PLUS_ONE: u32 = 1;
const MINUS_TWO: u32 = 2;

fn my_main(engine: &mut Engine<RegistryLookup>) {
    keep_mulligans(engine);
    reach_main_phase(engine, ME);
}

fn refusal(engine: &Engine<RegistryLookup>) -> Option<ForkRefusal> {
    engine.fork_for(ME, DeckKnowledge::Known, 5).err()
}

fn activate(engine: &mut Engine<RegistryLookup>, source: ObjectId, ability_index: u32) {
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap_or_else(|e| panic!("{ability_index} of {source:?} is refused: {e:?}"));
}

/// Nothing names an activator, and the fork is not refused over one.
#[track_caller]
fn names_no_activator(engine: &Engine<RegistryLookup>) {
    let choice = &engine.activation_target_choice;
    assert!(
        choice.controller.is_none()
            && choice.opponent.is_none()
            && choice.second_players.is_empty(),
        "an activator outlived its activation: {choice:?}"
    );
    assert_ne!(refusal(engine), Some(STALE));
}

/// Jace's +1 goes on the stack and resolves, and his −2 asks its target
/// first. While the target is asked, the activator is named and the fork is
/// refused, as for any activation being made. Once either ability is on the
/// stack, the activation is over: with Jace's ability waiting to resolve
/// and after it has, nothing names an activator and the game forks.
#[test]
fn a_loyalty_activation_lets_go_of_its_activator() {
    let mut engine = Duel::new(61, basic_forest())
        .battlefield(0, &[jace()])
        .battlefield(1, &[quiet_creature()])
        .start();
    my_main(&mut engine);
    let walker = on_battlefield(&engine, ME, jace()).expect("Jace");
    activate(&mut engine, walker, PLUS_ONE);
    assert!(!stack_is_empty(&engine), "the +1 is on the stack");
    names_no_activator(&engine);
    assert_eq!(
        refusal(&engine),
        None,
        "a stack with an ability on it forks"
    );
    pass_until(&mut engine, stack_is_empty);
    names_no_activator(&engine);
    assert_eq!(refusal(&engine), None);

    pass_until(&mut engine, |e| {
        e.state().turn.active == ME
            && e.state().turn.number > 1
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == ME)
    });
    let target = on_battlefield(&engine, THEM, quiet_creature()).expect("their Elves");
    activate(&mut engine, walker, MINUS_TWO);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the −2 asks its target, got {:?}", engine.pending())
    };
    assert!(options.contains(&target));
    assert_eq!(
        engine.activation_target_choice.controller,
        Some(ME),
        "the activator is named while its target is asked"
    );
    assert_eq!(
        refusal(&engine),
        Some(ForkRefusal::Continuation("pending_plan"))
    );
    engine
        .apply(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("their Elves");
    assert!(!stack_is_empty(&engine), "the −2 is on the stack");
    names_no_activator(&engine);
    assert_eq!(refusal(&engine), None);
}

/// Branch of Vitu-Ghazi cast face down with disguise and turned face up, a
/// special action taken through the activation door (CR 116.2b). It is no
/// activation and names no activator, neither while what turning it up
/// triggered waits on the stack nor once that has resolved.
#[test]
fn turning_a_permanent_face_up_names_no_activator() {
    let (branch, plains) = (named("Branch of Vitu-Ghazi"), named("Plains"));
    let mut engine = Duel::new(423, plains)
        .battlefield(0, &[plains; 6])
        .hand(0, &[branch])
        .start();
    my_main(&mut engine);
    cast_from_hand(&mut engine, ME, branch);
    pass_until(&mut engine, stack_is_empty);
    let face_down = on_battlefield(&engine, ME, branch).expect("the Branch");
    assert_eq!(refusal(&engine), Some(ForkRefusal::FaceDown));
    activate(&mut engine, face_down, crate::choice::TURN_FACE_UP);
    assert!(
        !engine
            .state()
            .object(face_down)
            .unwrap()
            .status
            .contains(crate::object::Status::FACE_DOWN),
        "turned face up"
    );
    names_no_activator(&engine);
    // What it triggered resolves, asking its colours as it does.
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        let (seat, answer) = match engine.pending() {
            Pending::Priority { player, .. } => (*player, PlayerAction::PassPriority),
            _ => answer_one(&engine).unwrap_or_else(|rest| panic!("unanswerable: {rest:?}")),
        };
        engine.apply(seat, answer).expect("an answer the kit found");
    }
    assert!(stack_is_empty(&engine), "resolved");
    names_no_activator(&engine);
    assert_eq!(refusal(&engine), None);
}

/// Walk-In Closet // Forgotten Cellar on the battlefield with both doors
/// locked, and one of them unlocked: a special action (CR 116.2m) taken
/// through the activation door, which names no activator.
#[test]
fn unlocking_a_door_names_no_activator() {
    let room = named("Walk-In Closet // Forgotten Cellar");
    let mut field = vec![basic_forest(); 8];
    field.push(room);
    let mut engine = Duel::new(67, basic_forest()).battlefield(0, &field).start();
    my_main(&mut engine);
    let id = on_battlefield(&engine, ME, room).expect("the Room");
    tap_all_mana(&mut engine, ME);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("my priority, got {:?}", engine.pending())
    };
    let Some(half) = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == id)
        .find_map(|&(_, index)| crate::choice::door_to_unlock(index))
    else {
        panic!("a door is offered: {:?}", legal.abilities)
    };
    activate(&mut engine, id, crate::choice::unlock_door(half));
    assert!(
        engine.state().object(id).unwrap().doors.is_unlocked(half),
        "unlocked"
    );
    names_no_activator(&engine);
    pass_until(&mut engine, stack_is_empty);
    names_no_activator(&engine);
    assert_eq!(refusal(&engine), None);
}

/// Runs `f` as the engine was: the activator named before the special
/// actions and kept after a loyalty ability ([`KEEP_STALE_TARGET_CHOICE`]).
fn as_before<T>(f: impl FnOnce() -> T) -> T {
    KEEP_STALE_TARGET_CHOICE.with(|keep| keep.set(true));
    let out = f();
    KEEP_STALE_TARGET_CHOICE.with(|keep| keep.set(false));
    out
}

/// What a pair of lockstep games reached.
#[derive(Default)]
struct Reached {
    decisions: usize,
    /// Decisions before which the engine as it was held an activator the
    /// engine as it is does not: the positions whose hash moved.
    moved: usize,
    /// Of those, the ones at priority that the engine as it is forks at and
    /// the engine as it was refused over the activator.
    forks_gained: usize,
    /// The answers after which a difference began, by kind.
    after: std::collections::BTreeMap<&'static str, usize>,
}

impl Reached {
    fn absorb(&mut self, other: Self) {
        self.decisions += other.decisions;
        self.moved += other.moved;
        self.forks_gained += other.forks_gained;
        for (kind, n) in other.after {
            *self.after.entry(kind).or_default() += n;
        }
    }
}

/// What kind of answer `action` is, for [`Reached::after`].
fn kind(engine: &Engine<RegistryLookup>, action: &PlayerAction) -> &'static str {
    match action {
        PlayerAction::ActivateAbility { ability_index, .. }
            if *ability_index == crate::choice::TURN_FACE_UP =>
        {
            "turn face up"
        }
        PlayerAction::ActivateAbility { ability_index, .. }
            if crate::choice::door_to_unlock(*ability_index).is_some() =>
        {
            "unlock a door"
        }
        PlayerAction::ActivateAbility { ability_index, .. }
            if *ability_index == crate::choice::PREPARED_CAST =>
        {
            "prepared cast"
        }
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } if engine.loyalty_cost(*source, *ability_index).is_some() => "loyalty ability",
        PlayerAction::ChooseTargets { .. } | PlayerAction::ChoosePlayer(_) => "a loyalty target",
        _ => "other",
    }
}

/// A position where the engine as it was (`then`) holds an activator the
/// engine as it is (`now`) does not: nothing is in flight, the field is the
/// whole difference in the print and in the hash, and the hash the engine
/// as it was wrote is the one a replay audit asks for
/// (`snapshot_hash_naming_stale_activator`). Whether the engine as it is
/// forks here where the engine as it was refused.
fn moved(
    now: &mut Engine<RegistryLookup>,
    then: &Engine<RegistryLookup>,
    differing: &[&str],
    at: &str,
    seed: u64,
) -> bool {
    let (old, new) = (
        &then.activation_target_choice,
        &now.activation_target_choice,
    );
    assert!(
        old.controller.is_some()
            && new.controller.is_none()
            && old.opponent == new.opponent
            && old.second_players == new.second_players,
        "{at}: {old:?} before, {new:?} now"
    );
    assert!(
        now.pending_plan.is_none() && now.activation_payment.is_none(),
        "{at}: the activator differs with an activation in flight"
    );
    // The light print shows the field beside the second targets, which the
    // caller found equal.
    assert_eq!(differing, ["activation_second_targets"], "{at}");
    assert_ne!(now.snapshot_hash(), then.snapshot_hash(), "{at}");
    let kept = std::mem::replace(
        &mut now.activation_target_choice,
        then.activation_target_choice.clone(),
    );
    assert_eq!(
        now.snapshot_hash(),
        then.snapshot_hash(),
        "{at}: the activator is the whole difference"
    );
    now.activation_target_choice = kept;
    let seat = then.activation_target_choice.controller.expect("checked");
    assert_eq!(
        now.snapshot_hash_naming_stale_activator(seat),
        then.snapshot_hash(),
        "{at}"
    );
    let Some(asked) = now.pending().asked() else {
        return false;
    };
    if !matches!(now.pending(), Pending::Priority { .. })
        || now.fork_for(asked, DeckKnowledge::Known, seed).is_err()
    {
        return false;
    }
    assert_eq!(
        then.fork_for(asked, DeckKnowledge::Known, seed).err(),
        Some(STALE),
        "{at}"
    );
    true
}

/// Deck `a` against deck `b` at `seed`, each seat's battlefield set by
/// `board`, played twice in lockstep to its end or `cap` decisions, as the
/// engine is and as it was, with the refusal sweep's seeded answers. Every
/// answer is taken or refused alike. Before each decision either the two
/// hold the same `activation_target_choice` and have the same snapshot hash
/// and light print, or the engine as it was holds an activator the engine
/// as it is does not, with no activation in flight; then the hashes differ,
/// and putting that activator into the engine as it is makes them equal, so
/// the field is the whole difference.
fn lockstep(
    a: &baylee_cards::decks::LoadedDeck,
    b: &baylee_cards::decks::LoadedDeck,
    seed: u64,
    cap: usize,
    board: fn(&mut baylee_core::preset::GamePreset),
) -> Reached {
    let mut preset = baylee_cards::decks::preset_for(seed, a, b);
    board(&mut preset);
    let mut now = Engine::new(&preset, RegistryLookup).expect("house decks start a game");
    let mut then = Engine::new(&preset, RegistryLookup).expect("the same game");
    let mut dice = Dice(seed ^ 0x5EED);
    let mut reached = Reached::default();
    for decision in 0..cap {
        let at = format!(
            "{} v {} seed {seed} decision {decision} turn {}",
            a.name,
            b.name,
            now.state().turn.number
        );
        assert_eq!(
            now.activation_second_targets, then.activation_second_targets,
            "{at}"
        );
        let differing = now.fingerprint_light().differing(&then.fingerprint_light());
        if format!("{:?}", now.activation_target_choice)
            == format!("{:?}", then.activation_target_choice)
        {
            assert_eq!(now.snapshot_hash(), then.snapshot_hash(), "{at}");
            assert!(
                differing.is_empty(),
                "{at}: the twins differ in {differing:?}"
            );
        } else {
            reached.moved += 1;
            reached.forks_gained += usize::from(moved(&mut now, &then, &differing, &at, seed));
        }
        let pending = now.pending().clone();
        let Some(asked) = pending.asked() else {
            break; // game over
        };
        reached.decisions += 1;
        let alike = format!("{:?}", now.activation_target_choice)
            == format!("{:?}", then.activation_target_choice);
        let mut answered = false;
        for action in offered(&now, &pending, &mut dice) {
            let what = kind(&now, &action);
            let taken_now = now.apply(asked, action.clone());
            let taken_then = as_before(|| then.apply(asked, action.clone()));
            assert_eq!(
                taken_now.is_ok(),
                taken_then.is_ok(),
                "{at}: {action:?} is {taken_now:?} now and was {taken_then:?}"
            );
            if taken_now.is_ok() {
                if alike
                    && format!("{:?}", now.activation_target_choice)
                        != format!("{:?}", then.activation_target_choice)
                {
                    *reached.after.entry(what).or_default() += 1;
                }
                answered = true;
                break;
            }
        }
        assert!(answered, "{at}: nothing answered {pending:?}");
    }
    reached
}

/// A deck that reaches every path that left an activator standing: The
/// Theorist, Jace Beleren (loyalty), Branch of Vitu-Ghazi (cast face down
/// with disguise and turned face up) and Walk-In Closet (a door to
/// unlock). The house decks hold planeswalkers too, but the seeded driver
/// rarely casts one: nine house games reached none.
const STALE_DECK: &str = "[deck:Stale]
14 Island
12 Forest
4 The Theorist, Jace Beleren
6 Branch of Vitu-Ghazi
4 Walk-In Closet // Forgotten Cellar
4 Llanowar Elves
";

/// Each seat begins with a Jace and a Room on the battlefield beside six
/// lands, so the games activate loyalty and unlock doors from the first
/// turn rather than once in a while.
fn stale_board(preset: &mut baylee_core::preset::GamePreset) {
    const BOARD: [&str; 8] = [
        "The Theorist, Jace Beleren",
        "Walk-In Closet // Forgotten Cellar",
        "Island",
        "Island",
        "Island",
        "Forest",
        "Forest",
        "Forest",
    ];
    for seat in &mut preset.seats {
        seat.starting_battlefield = BOARD
            .iter()
            .map(|name| {
                let card = named(name);
                *seat
                    .deck
                    .iter()
                    .find(|e| e.card == card)
                    .expect("the deck lists every card of its board")
            })
            .collect();
    }
}

/// The hashes the change moves, and only those. The deck above plays
/// itself, and each house deck its neighbour, as the engine is and as it
/// was: the same answers are taken, and the two hash alike before every
/// decision except where the engine as it was still held the activator of
/// a loyalty ability or of a special action already done. Those hashes
/// move, by that field alone, and those positions now fork.
#[test]
fn only_positions_holding_a_stale_activator_hash_differently() {
    let stale = baylee_cards::decks::load_acceptance(STALE_DECK, "Stale")
        .unwrap_or_else(|e| panic!("the stale deck: {e}"));
    let mut total = Reached::default();
    for seed in 0..6 {
        total.absorb(lockstep(&stale, &stale, 7_200 + seed, 800, stale_board));
    }
    // Each path is reached, so the comparison holds for each: measured on
    // 10 November 2026 at 48 loyalty abilities, 10 loyalty targets
    // answered, 6 permanents turned face up and 4 doors unlocked, which
    // moved 364 hashes and gained 290 forks.
    for path in ["loyalty ability", "turn face up", "unlock a door"] {
        assert!(
            total.after.contains_key(path),
            "no game left an activator standing after a {path}: {:?}, \
             {} decisions, {} moved, {} forks gained",
            total.after,
            total.decisions,
            total.moved,
            total.forks_gained
        );
    }
    assert!(
        total.forks_gained > 0,
        "{} moved, no fork gained",
        total.moved
    );
    let decks: Vec<_> = DECKS.iter().map(|(f, n)| house_deck(f, n)).collect();
    for a in 0..decks.len() {
        let b = (a + 1) % decks.len();
        lockstep(&decks[a], &decks[b], 7_100 + a as u64, 600, |_| {});
    }
}
