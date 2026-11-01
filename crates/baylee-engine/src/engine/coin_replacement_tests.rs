//! Coin flips under a replacement that flips more coins and ignores some
//! (CR 614.1a; Krark's Thumb: "If you would flip a coin, instead flip two
//! coins and ignore one"): the instruction that flipped goes on from the
//! result kept, once, and then the next instruction runs.

use super::testkit::*;
use super::*;
use crate::choice::YesNoPrompt;
use crate::event::GameEvent;
use baylee_core::ids::CardIndex;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn krark_s_thumb() -> CardIndex {
    card_index("a97c8482-775c-4f11-9872-25b4f9bfcb1a")
}

fn mana_clash() -> CardIndex {
    card_index("a2f5a5fd-14dc-48e8-bdb0-984a83288023")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Whether the Thumb's question, which of the caster's two coins to ignore,
/// is the one standing.
fn thumb_asks(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            player: P0,
            prompt: YesNoPrompt::Generic,
            ..
        }
    )
}

/// Every coin flip the journal holds, in order, as (who, came up heads).
fn flips(e: &Engine<RegistryLookup>) -> Vec<(PlayerId, bool)> {
    e.state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::CoinFlipped { player, won } => Some((player, won)),
            _ => None,
        })
        .collect()
}

/// Mana Clash ("You and target opponent each flip a coin … Repeat this
/// process until both players' coins come up heads on the same flip") cast
/// by the Thumb's controller: each round is the caster's two coins, one of
/// them kept, then the opponent's one; it repeats until both kept coins are
/// heads, each tails costs its player 1 life, and then the spell is done.
///
/// The contest asked about used to run its own instruction again after
/// every answer, so the Thumb was asked again for ever and the spell never
/// resolved: here that is the cap on the answers.
#[test]
fn mana_clash_under_krarks_thumb_ends_on_two_kept_heads() {
    let mut longest = 0;
    for seed in 1..=12 {
        let mut e = Duel::new(seed, basic_forest())
            .battlefield(0, &[krark_s_thumb(), mountain()])
            .hand(0, &[mana_clash()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        cast_from_hand(&mut e, P0, mana_clash());
        // The target opponent, however the announcement asks for it.
        match e.pending() {
            Pending::ChooseTargets { .. } => e
                .apply(
                    P0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![P1],
                    },
                )
                .unwrap(),
            Pending::ChoosePlayer { .. } => e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap(),
            _ => {}
        }
        let mut answers = 0_usize;
        loop {
            pass_until(&mut e, |e| {
                thumb_asks(e)
                    || (stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }))
            });
            if !thumb_asks(&e) {
                break;
            }
            answers += 1;
            assert!(answers <= 64, "seed {seed}: the contest never ended");
            // Ignore the first coin of the pair: the second is kept.
            e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        }
        let flips = flips(&e);
        assert_eq!(
            flips.len(),
            answers * 3,
            "seed {seed}: one question and three coins a round: {flips:?}"
        );
        let rounds: Vec<(bool, bool)> = flips
            .chunks(3)
            .map(|round| {
                assert_eq!(
                    (round[0].0, round[1].0, round[2].0),
                    (P0, P0, P1),
                    "seed {seed}: the caster's two coins, then the opponent's"
                );
                (round[1].1, round[2].1)
            })
            .collect();
        let (last, earlier) = rounds.split_last().expect("at least one round");
        assert_eq!(
            *last,
            (true, true),
            "seed {seed}: it ends on two heads kept"
        );
        assert!(
            earlier.iter().all(|(mine, theirs)| !(*mine && *theirs)),
            "seed {seed}: no earlier round kept two heads"
        );
        let mine = rounds.iter().filter(|(heads, _)| !heads).count();
        let theirs = rounds.iter().filter(|(_, heads)| !heads).count();
        assert_eq!(
            e.state().players[0].life,
            20 - i32::try_from(mine).unwrap(),
            "seed {seed}"
        );
        assert_eq!(
            e.state().players[1].life,
            20 - i32::try_from(theirs).unwrap(),
            "seed {seed}"
        );
        assert!(in_graveyard(&e, P0, mana_clash()).is_some(), "seed {seed}");
        longest = longest.max(rounds.len());
    }
    assert!(longest > 1, "some seed had to repeat the process");
}
