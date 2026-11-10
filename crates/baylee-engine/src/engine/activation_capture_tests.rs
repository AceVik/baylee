//! What an activation captures for its push ends with the activation.
//!
//! `Engine::activating_abilities` (the source's ability list, read before a
//! cost can move the source) and `Engine::activating_grant` (a granted
//! ability's definition) are filled as an activation starts, for the push of
//! that activation's own ability. Three paths end an activation: the push,
//! which takes them; a mana ability, which resolves at once without the
//! stack (CR 605.3b); and a reversal after an answer (`reverse_activation`).
//! The last two used to leave both standing until the next push or the next
//! activation took or overwrote them, and every fork in between was refused
//! as an activation in progress (`ForkRefusal::Continuation`), 42% of one
//! search run's roots.
//!
//! The first tests reach each stale case and one activation still being
//! made; the last plays house games twice, with the release and without it,
//! and holds the two to the same game.

use super::abilities::KEEP_STALE_CAPTURE;
use super::refusal_tests::{DECKS, Dice, house_deck, offered};
use super::testkit::*;
use super::*;
use crate::engine::{DeckKnowledge, ForkRefusal};
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

const ME: PlayerId = PlayerId::new(0);

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn llanowar_elves() -> CardIndex {
    quiet_creature()
}

fn sol_ring() -> CardIndex {
    quiet_artifact()
}

/// "Creatures you control have '{T}: Add one mana of any color.'"
fn cryptolith_rite() -> CardIndex {
    baylee_cards::decks::by_name("Cryptolith Rite").expect("the registry holds the card")
}

/// "{2}, {T}, Sacrifice a creature: Draw a card."
fn phyrexian_vault() -> CardIndex {
    card_index("b628150b-08a1-4ea3-978d-60255dfb0b7e")
}

/// My first main phase with `field` on my battlefield, holding priority.
fn my_main(seed: u64, field: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, forest()).battlefield(0, field).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, ME);
    engine
}

fn priority_for(engine: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
}

fn pool(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u64 {
    engine.state().players[seat.get() as usize]
        .mana_pool
        .total()
}

fn refusal(engine: &Engine<RegistryLookup>) -> Option<ForkRefusal> {
    engine.fork_for(ME, DeckKnowledge::Known, 5).err()
}

/// That `engine` forks for me, and that the fork goes on as the game does:
/// the same question now, and the same one again after both have passed.
#[track_caller]
fn forks_and_continues(engine: &mut Engine<RegistryLookup>) {
    let mut fork = engine
        .fork_for(ME, DeckKnowledge::Known, 5)
        .unwrap_or_else(|refusal| panic!("nothing is in flight, and the fork says {refusal}"));
    for game in [&mut *engine, &mut fork] {
        assert!(
            game.activating_abilities.is_none() && game.activating_grant.is_none(),
            "no capture outlives its activation"
        );
    }
    assert_eq!(
        format!("{:?}", fork.pending()),
        format!("{:?}", engine.pending()),
        "the fork asks what the game asks"
    );
    for game in [&mut *engine, &mut fork] {
        game.apply(ME, PlayerAction::PassPriority).unwrap();
    }
    assert_eq!(
        format!("{:?}", fork.pending()),
        format!("{:?}", engine.pending())
    );
    assert_eq!(
        fork.state().zones.list(ZoneLocation::Battlefield),
        engine.state().zones.list(ZoneLocation::Battlefield)
    );
}

/// Sol Ring tapped for mana in my main phase. A mana ability does not go on
/// the stack and resolves at once (CR 605.3b), so with its mana in my pool
/// nothing of the activation is in flight, and the game forks there as it
/// does at any other priority. The list the activation read for a push it
/// never makes used to stay behind and refuse the fork as
/// `Continuation("activating_abilities")`.
#[test]
fn a_mana_ability_leaves_nothing_in_flight() {
    let mut engine = my_main(41, &[sol_ring()]);
    let ring = on_battlefield(&engine, ME, sol_ring()).expect("my Sol Ring");
    assert_eq!(refusal(&engine), None, "a settled main phase forks");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("my priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ring, 0)),
        "Sol Ring's mana ability is offered: {:?}",
        legal.abilities
    );
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: ring,
                ability_index: 0,
            },
        )
        .expect("Sol Ring taps for mana");
    assert!(priority_for(&engine, ME), "{:?}", engine.pending());
    assert!(stack_is_empty(&engine), "a mana ability uses no stack");
    assert_eq!(pool(&engine, ME), 2, "{{C}}{{C}} is in my pool");
    forks_and_continues(&mut engine);
}

/// The same for a granted mana ability that asks its colour as it resolves
/// (Cryptolith Rite on Llanowar Elves). The question suspends the
/// resolution, and that return used to skip the line that dropped the
/// granted definition, so `activating_grant` stayed behind as well.
#[test]
fn a_granted_mana_ability_asking_its_colour_leaves_nothing_in_flight() {
    let mut engine = my_main(43, &[cryptolith_rite(), llanowar_elves()]);
    let elves = on_battlefield(&engine, ME, llanowar_elves()).expect("my Elves");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("my priority, got {:?}", engine.pending())
    };
    let Some(&(_, granted)) = legal
        .abilities
        .iter()
        .find(|(source, index)| *source == elves && crate::choice::granted_slot(*index).is_some())
    else {
        panic!("the Rite's ability is offered: {:?}", legal.abilities)
    };
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: elves,
                ability_index: granted,
            },
        )
        .expect("the Elves tap for any colour");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("the colour is asked, got {:?}", engine.pending())
    };
    assert!(
        matches!(refusal(&engine), Some(ForkRefusal::Continuation(_))),
        "the colour question is a resolution in flight"
    );
    engine
        .apply(ME, PlayerAction::ChooseColor(options[0]))
        .expect("a colour offered");
    assert!(priority_for(&engine, ME), "{:?}", engine.pending());
    assert_eq!(pool(&engine, ME), 1);
    forks_and_continues(&mut engine);
}

/// Phyrexian Vault answered with a creature that is gone by the time the
/// answer is paid, which the harness arranges as `reversal_tests` does: the
/// activation is reversed (CR 734.1) and priority is mine again, with
/// nothing in flight. The reversal cleared every field of the activation
/// but the capture.
#[test]
fn a_reversed_activation_leaves_nothing_in_flight() {
    let mut engine = my_main(
        47,
        &[
            phyrexian_vault(),
            forest(),
            forest(),
            llanowar_elves(),
            llanowar_elves(),
        ],
    );
    let vault = on_battlefield(&engine, ME, phyrexian_vault()).expect("the Vault");
    let forests = all_on_battlefield(&engine, forest());
    tap_mana_where(&mut engine, ME, |id| forests.contains(&id));
    assert_eq!(pool(&engine, ME), 2);
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: vault,
                ability_index: 0,
            },
        )
        .expect("the Vault activates");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    let gone = options[0];
    let state = engine
        .dev_state_mut(ME)
        .expect("the harness sets boards up");
    assert!(state.zones.remove(gone, ZoneLocation::Battlefield));
    assert!(state.arena.remove(gone).is_some());
    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![gone],
            },
        )
        .expect("an answer the question offered is taken");
    assert!(
        priority_for(&engine, ME),
        "reversed: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "nothing was activated");
    forks_and_continues(&mut engine);
}

/// Every permanent on the battlefield that is `card`.
fn all_on_battlefield(engine: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// "{X}: This artifact becomes an X/X Construct artifact creature until end
/// of turn."
fn chimeric_staff() -> CardIndex {
    card_index("771f5e29-b4d1-4466-8510-ce3f89ad7173")
}

/// Pays the open window by tapping every Forest of mine into it and
/// passing until it closes.
#[track_caller]
fn pay_window_with_forests(engine: &mut Engine<RegistryLookup>) {
    for land in all_on_battlefield(engine, forest()) {
        if engine.payment_window().is_none() {
            break;
        }
        engine
            .apply(ME, PlayerAction::ActivateManaAbility { source: land })
            .expect("a Forest pays into the window");
    }
    for _ in 0..4 {
        if engine.payment_window().is_none() {
            break;
        }
        engine.apply(ME, PlayerAction::PassPriority).unwrap();
    }
    assert!(
        engine.payment_window().is_none(),
        "paid: {:?}",
        engine.pending()
    );
}

/// An activation still being made is refused as it always was, at each of
/// its stages, and the list it read before its cost stays where its push
/// will find it: while Phyrexian Vault asks which creature to sacrifice
/// (the plan holds the activation), while Chimeric Staff asks its X (the
/// same) and while its {X} is paid in a window (the window holds the
/// activation, list and all). Once an ability is on the stack, its push has
/// taken the list and the game forks.
#[test]
fn an_activation_in_flight_is_still_refused() {
    let field = [phyrexian_vault(), forest(), forest(), llanowar_elves()];
    let mut engine = my_main(53, &field);
    let vault = on_battlefield(&engine, ME, phyrexian_vault()).expect("the Vault");
    let forests = all_on_battlefield(&engine, forest());
    tap_mana_where(&mut engine, ME, |id| forests.contains(&id));
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: vault,
                ability_index: 0,
            },
        )
        .expect("the Vault activates");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(
        refusal(&engine),
        Some(ForkRefusal::Continuation("pending_plan"))
    );
    assert_eq!(
        engine.activating_abilities.as_ref().map(|(id, _)| *id),
        Some(vault),
        "the list read before the cost is held for the push"
    );
    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the Elves to sacrifice");
    assert!(
        !stack_is_empty(&engine),
        "the Vault's ability is on the stack"
    );
    assert!(priority_for(&engine, ME), "{:?}", engine.pending());
    assert_eq!(refusal(&engine), None, "the push took the list");

    let mut engine = my_main(59, &[chimeric_staff(), forest(), forest()]);
    let staff = on_battlefield(&engine, ME, chimeric_staff()).expect("the Staff");
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: staff,
                ability_index: 0,
            },
        )
        .expect("the Staff activates");
    assert!(
        matches!(engine.pending(), Pending::ChooseNumber { .. }),
        "X is announced first: {:?}",
        engine.pending()
    );
    assert_eq!(
        refusal(&engine),
        Some(ForkRefusal::Continuation("pending_plan"))
    );
    engine
        .apply(ME, PlayerAction::ChooseNumber(2))
        .expect("X = 2");
    assert!(
        engine.payment_window().is_some(),
        "{{X}} is paid in a window: {:?}",
        engine.pending()
    );
    assert_eq!(
        refusal(&engine),
        Some(ForkRefusal::Continuation("mana_window"))
    );
    pay_window_with_forests(&mut engine);
    assert!(
        !stack_is_empty(&engine),
        "the Staff's ability is on the stack"
    );
    assert!(priority_for(&engine, ME), "{:?}", engine.pending());
    assert_eq!(refusal(&engine), None, "the push took the list");
}

/// Runs `f` with the release switched off on this thread
/// ([`KEEP_STALE_CAPTURE`]): the engine as it was before it.
fn as_before<T>(f: impl FnOnce() -> T) -> T {
    KEEP_STALE_CAPTURE.with(|keep| keep.set(true));
    let out = f();
    KEEP_STALE_CAPTURE.with(|keep| keep.set(false));
    out
}

/// The two fields the release empties, the only ones twins may differ in.
const CAPTURE: [&str; 2] = ["activating_abilities", "activating_grant"];

/// What a pair of twin games reached.
#[derive(Default)]
struct Twins {
    decisions: usize,
    /// Priority decisions the engine as it is forked at.
    forks: usize,
    /// Of those, the ones the engine as it was refused, for a capture left
    /// standing: the forks the release gained.
    gained: usize,
}

/// Deck `a` against deck `b` at `seed`, played twice in lockstep to its end
/// or `cap` decisions: as the engine is, and as it was before the release.
/// Both are put the same seeded answers (the refusal sweep's driver), and
/// each answer has to be taken by both or refused by both. Before every
/// decision the two have the same snapshot hash, and their light print
/// (every field but the arena, which the hash reads, and the base cache and
/// names, which follow from the deck lists) differs in the capture fields at
/// most. Where the engine as it is forks, the engine as it was forks too or
/// is refused over the capture, and for nothing else. Panics at the first
/// difference.
fn twins(
    a: &baylee_cards::decks::LoadedDeck,
    b: &baylee_cards::decks::LoadedDeck,
    seed: u64,
    cap: usize,
) -> Twins {
    let preset = baylee_cards::decks::preset_for(seed, a, b);
    let mut now = Engine::new(&preset, RegistryLookup).expect("house decks start a game");
    let mut then = Engine::new(&preset, RegistryLookup).expect("the same game");
    let mut dice = Dice(seed ^ 0x5EED);
    let mut reached = Twins::default();
    for decision in 0..cap {
        let at = format!(
            "{} v {} seed {seed} decision {decision} turn {}",
            a.name,
            b.name,
            now.state().turn.number
        );
        assert_eq!(now.snapshot_hash(), then.snapshot_hash(), "{at}");
        let differing: Vec<&str> = now
            .fingerprint_light()
            .differing(&then.fingerprint_light())
            .into_iter()
            .filter(|field| !CAPTURE.contains(field))
            .collect();
        assert!(
            differing.is_empty(),
            "{at}: the twins differ in {differing:?}"
        );
        let pending = now.pending().clone();
        let Some(asked) = pending.asked() else {
            break; // game over
        };
        reached.decisions += 1;
        if matches!(pending, Pending::Priority { .. }) {
            let policy = seed ^ decision as u64;
            match now.fork_for(asked, DeckKnowledge::Known, policy) {
                Ok(_) => {
                    reached.forks += 1;
                    match then.fork_for(asked, DeckKnowledge::Known, policy) {
                        Ok(_) => {}
                        Err(ForkRefusal::Continuation(field)) if CAPTURE.contains(&field) => {
                            reached.gained += 1;
                        }
                        Err(other) => panic!("{at}: forks now, and was refused for {other}"),
                    }
                }
                Err(ForkRefusal::Continuation(field)) if CAPTURE.contains(&field) => {
                    panic!("{at}: refused a fork for {field} with nothing in flight")
                }
                Err(_) => {}
            }
        }
        let mut answered = false;
        for action in offered(&now, &pending, &mut dice) {
            let taken_now = now.apply(asked, action.clone());
            let taken_then = as_before(|| then.apply(asked, action.clone()));
            assert_eq!(
                taken_now.is_ok(),
                taken_then.is_ok(),
                "{at}: {action:?} is {taken_now:?} now and was {taken_then:?}"
            );
            if taken_now.is_ok() {
                answered = true;
                break;
            }
        }
        assert!(answered, "{at}: nothing answered {pending:?}");
    }
    reached
}

/// Releasing the capture changes nothing in a game but the fork. Every
/// house deck plays its neighbour twice, as the engine is and as it was:
/// the same answers are taken, every snapshot hash is the same (the capture
/// is not hashed), and the engines differ in the two capture fields and in
/// no other. The engine as it was refuses forks over a capture left
/// standing in these games, so the comparison reaches the change; the
/// engine as it is never does.
#[test]
fn releasing_the_capture_changes_nothing_but_the_fork() {
    let decks: Vec<_> = DECKS.iter().map(|(f, n)| house_deck(f, n)).collect();
    let mut total = Twins::default();
    for a in 0..decks.len() {
        let b = (a + 1) % decks.len();
        let reached = twins(&decks[a], &decks[b], 7_000 + a as u64, 1_200);
        total.decisions += reached.decisions;
        total.forks += reached.forks;
        total.gained += reached.gained;
    }
    assert!(
        total.gained > 0,
        "no capture was ever left standing, so the twins compared nothing: \
         {} decisions, {} forks",
        total.decisions,
        total.forks
    );
}
