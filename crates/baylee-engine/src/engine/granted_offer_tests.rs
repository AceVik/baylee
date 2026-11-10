//! "Can this seat make mana?" is asked of the list a payment window opens
//! with, and of no other.
//!
//! `LegalActions::has_mana_source` reads any entry of `abilities` as a mana
//! source, which is true only of a list a payment window narrowed. Four
//! places asked it of another list. The granted-action offer asked it while
//! `compute_legal` was still building the offer, when `abilities` held
//! every activation, also one no mana paid: a seat with every land tapped
//! was offered Nafs Asp's "pay {1}", and the window found nothing to pay
//! with and refused it (deal 359 of the v6 priority probe). The free-cast
//! offers (linked and immediate free plays, a transformed back face, a
//! commanded cast) asked it of the finished, un-narrowed offer, where a
//! Prodigal Sorcerer's tap counts as mana to come. All four now ask the
//! narrowed list (`settle_granted_offers`, `free_cast_makes_mana`).
//!
//! Neither the legal list nor the question is in `Engine::snapshot_hash`,
//! so no hash moves. The games below play the change in lockstep against
//! the engine as it was (`ASK_MANA_AS_BEFORE`): the same answers are taken
//! or refused alike, the hashes are equal before every decision, and the
//! questions differ only where the engine as it was offered a granted
//! action that it then refused.

use super::granted::ASK_MANA_AS_BEFORE;
use super::refusal_tests::{DECKS, Dice, house_deck, offered};
use super::testkit::*;
use super::*;
use crate::choice::LegalActions;
use baylee_core::ids::{CardIndex, GrantedActionId};

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).expect("the registry holds the card")
}

/// Runs `f` as the engine was.
fn as_before<T>(f: impl FnOnce() -> T) -> T {
    ASK_MANA_AS_BEFORE.with(|before| before.set(true));
    let out = f();
    ASK_MANA_AS_BEFORE.with(|before| before.set(false));
    out
}

/// What a pair of lockstep games reached.
#[derive(Default, Debug)]
struct Reached {
    decisions: usize,
    /// Positions where the engine as it was offered granted actions the
    /// engine as it is does not.
    differed: usize,
    /// The granted actions offered only as it was, each refused.
    refused: usize,
    /// Granted actions taken in the games.
    taken: usize,
}

impl Reached {
    fn absorb(&mut self, other: &Self) {
        self.decisions += other.decisions;
        self.differed += other.differed;
        self.refused += other.refused;
        self.taken += other.taken;
    }
}

fn grants(legal: &LegalActions) -> Vec<GrantedActionId> {
    legal.granted_actions.iter().map(|offer| offer.id).collect()
}

/// Where the questions of `now` and `then` differ: they must both be
/// priority, alike but for granted actions `then` offers on top of
/// `now`'s, and `then` must refuse each of those and stay as it was. The
/// extra grants it refused.
fn wrong_offers(
    now: &Engine<RegistryLookup>,
    then: &mut Engine<RegistryLookup>,
    at: &str,
) -> usize {
    let (
        Pending::Priority {
            player,
            legal: legal_now,
        },
        Pending::Priority {
            legal: legal_then, ..
        },
    ) = (now.pending().clone(), then.pending().clone())
    else {
        panic!(
            "{at}: the questions differ outside priority: now {:?}, then {:?}",
            now.pending(),
            then.pending()
        )
    };
    let kept = grants(&legal_now);
    let extra: Vec<GrantedActionId> = grants(&legal_then)
        .into_iter()
        .filter(|id| !kept.contains(id))
        .collect();
    let mut trimmed = (*legal_then).clone();
    trimmed
        .granted_actions
        .retain(|offer| kept.contains(&offer.id));
    assert_eq!(
        format!("{trimmed:?}"),
        format!("{legal_now:?}"),
        "{at}: the lists differ in more than granted actions offered then"
    );
    assert!(!extra.is_empty(), "{at}: now offers a grant then did not");
    for id in &extra {
        let before = then.fingerprint_light();
        let taken = as_before(|| then.apply(player, PlayerAction::TakeGrantedAction { id: *id }));
        assert!(
            taken.is_err(),
            "{at}: the engine as it was took {id:?}, which is offered no more"
        );
        assert!(
            before.differing(&then.fingerprint_light()).is_empty(),
            "{at}: a refused grant moved the engine as it was"
        );
    }
    extra.len()
}

/// Deck `a` against deck `b` at `seed`, each seat's battlefield set by
/// `board`, played twice in lockstep to its end or `cap` decisions, as the
/// engine is and as it was, with the refusal sweep's seeded answers and,
/// at every third priority, the granted actions offered tried first.
fn lockstep(
    a: &baylee_cards::decks::LoadedDeck,
    b: &baylee_cards::decks::LoadedDeck,
    seed: u64,
    cap: usize,
    board: fn(&mut baylee_core::preset::GamePreset),
) -> Reached {
    let mut preset = baylee_cards::decks::preset_for(seed, a, b);
    board(&mut preset);
    let mut now = Engine::new(&preset, RegistryLookup).expect("the decks start a game");
    let mut then = as_before(|| Engine::new(&preset, RegistryLookup)).expect("the same game");
    let mut dice = Dice(seed ^ 0x5EED);
    let mut reached = Reached::default();
    for decision in 0..cap {
        let at = format!(
            "{} v {} seed {seed} decision {decision} turn {}",
            a.name,
            b.name,
            now.state().turn.number
        );
        assert_eq!(now.snapshot_hash(), then.snapshot_hash(), "{at}");
        if format!("{:?}", now.pending()) != format!("{:?}", then.pending()) {
            reached.differed += 1;
            reached.refused += wrong_offers(&now, &mut then, &at);
        }
        let differing: Vec<&str> = now
            .fingerprint_light()
            .differing(&then.fingerprint_light())
            .into_iter()
            .filter(|field| *field != "pending")
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
        let mut answers = offered(&now, &pending, &mut dice);
        if let Pending::Priority { legal, .. } = &pending
            && decision % 3 == 0
        {
            let take = grants(legal)
                .into_iter()
                .map(|id| PlayerAction::TakeGrantedAction { id });
            answers.splice(0..0, take);
        }
        let mut answered = false;
        for action in answers {
            let taken_now = now.apply(asked, action.clone());
            let taken_then = as_before(|| then.apply(asked, action.clone()));
            assert_eq!(
                taken_now.is_ok(),
                taken_then.is_ok(),
                "{at}: {action:?} is {taken_now:?} now and was {taken_then:?}"
            );
            if taken_now.is_ok() {
                if matches!(action, PlayerAction::TakeGrantedAction { .. }) {
                    reached.taken += 1;
                }
                answered = true;
                break;
            }
        }
        assert!(answered, "{at}: nothing answered {pending:?}");
    }
    reached
}

/// Nafs Asp to run up "pay {1}" debts, Prodigal Sorcerer for an activation
/// that makes no mana, Guardian Angel for a "pay {1}" of its own, and few
/// lands, so a debt often stands while every land is tapped or none is in
/// play.
const DEBT_DECK: &str = "[deck:Debts]
8 Forest
6 Island
4 Plains
14 Nafs Asp
14 Prodigal Sorcerer
4 Guardian Angel
";

/// Each seat begins with two Asps and a Sorcerer and no land, so the Asps
/// attack from the first turn and the debts come due before lands do.
fn debt_board(preset: &mut baylee_core::preset::GamePreset) {
    const BOARD: [&str; 3] = ["Nafs Asp", "Nafs Asp", "Prodigal Sorcerer"];
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

/// The change moves no hash and no answer: the debt deck plays itself and
/// each house deck its neighbour, as the engine is and as it was. Before
/// every decision the two hash alike; their questions differ only where the
/// engine as it was offered a granted action, and it refused each of those.
#[test]
fn only_granted_actions_nothing_could_pay_leave_the_offer() {
    let debts = baylee_cards::decks::load_acceptance(DEBT_DECK, "Debts")
        .unwrap_or_else(|e| panic!("the debt deck: {e}"));
    let mut total = Reached::default();
    for seed in 0..6 {
        total.absorb(&lockstep(&debts, &debts, 7_300 + seed, 800, debt_board));
    }
    assert!(
        total.refused > 0 && total.taken > 0,
        "the debt games reached no wrong offer or took no grant: {total:?}"
    );
    eprintln!("debt deck: {total:?}");
    let decks: Vec<_> = DECKS.iter().map(|(f, n)| house_deck(f, n)).collect();
    let mut house = Reached::default();
    for a in 0..decks.len() {
        let b = (a + 1) % decks.len();
        house.absorb(&lockstep(
            &decks[a],
            &decks[b],
            7_400 + a as u64,
            600,
            |_| {},
        ));
    }
    eprintln!("house decks: {house:?}");
}
