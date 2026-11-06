//! The suggestion as a player meets it: real cards where a card says it
//! best, bare facts where only the shape matters.

use super::facts::{self, Change, Deeds, Facts, Reading, Stuff, Total};
use super::memory::TriggerMemory;
use super::{Reason, Standing, Table, TriggerKey, Waiting, suggest};
use baylee_cards::dsl::AbilityDef;
use baylee_core::ids::{AbilityRef, CardIndex, ObjectId, PlayerId};

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);
const THIRD: PlayerId = PlayerId::new(2);

fn standing(seat: PlayerId, life: i32) -> Standing {
    Standing {
        seat,
        life,
        poison: 0,
        library: 40,
        ally: seat == ME,
    }
}

fn table(mine: i32, theirs: i32) -> Table {
    Table {
        me: ME,
        players: vec![standing(ME, mine), standing(THEM, theirs)],
    }
}

fn key(card: u32) -> TriggerKey {
    TriggerKey::Card(AbilityRef::new(CardIndex::new(card), 0))
}

fn trigger(card: u32, facts: Facts) -> Waiting {
    Waiting {
        key: Some(key(card)),
        source: None,
        twin: card,
        facts,
    }
}

fn doing(deeds: Deeds) -> Facts {
    Facts {
        deeds,
        ..Facts::default()
    }
}

fn life(who: PlayerId, by: i32) -> Facts {
    let mut facts = doing(match (by < 0, who == ME) {
        (true, true) => Deeds::HURTS_ME,
        (true, false) => Deeds::HURTS_OPPONENT,
        (false, _) => Deeds::HEALS_ME,
    });
    facts.changes.push(Change {
        who,
        what: Total::Life(by),
    });
    facts
}

/// The card of that name, and its first triggered ability.
fn printed(name: &str) -> (TriggerKey, &'static AbilityDef) {
    let card = baylee_cards::all()
        .find(|card| card.faces[0].name == name)
        .unwrap_or_else(|| panic!("{name} is in the pool"));
    let (index, ability) = card
        .abilities
        .iter()
        .enumerate()
        .find(|(_, ability)| {
            matches!(
                ability,
                AbilityDef::Triggered { .. } | AbilityDef::ModalTriggered { .. }
            )
        })
        .unwrap_or_else(|| panic!("{name} has a trigger"));
    let index = u32::try_from(index).expect("few abilities");
    (
        TriggerKey::Card(AbilityRef::new(card.index, index)),
        ability,
    )
}

/// A real card's trigger, as this player reads it, about that player's event.
fn real(name: &str, event_player: Option<PlayerId>) -> Waiting {
    let (key, ability) = printed(name);
    let opponents = [THEM];
    let reading = Reading {
        me: ME,
        opponents: &opponents,
        event_player,
        event_amount: None,
    };
    Waiting {
        key: Some(key),
        source: None,
        twin: event_player.map_or(0, |seat| u32::from(seat.get()) + 1),
        facts: facts::of_ability(ability, &reading),
    }
}

/// Who falls first down this order: what a test of the game's end checks,
/// so it does not lean on how the search found the order.
fn first_to_fall(waiting: &[Waiting], order: &[usize], table: &Table) -> Option<PlayerId> {
    let mut totals: Vec<(PlayerId, i32)> = table.players.iter().map(|p| (p.seat, p.life)).collect();
    for &index in order {
        for change in &waiting[index].facts.changes {
            if let Total::Life(by) = change.what {
                let entry = totals.iter_mut().find(|t| t.0 == change.who)?;
                entry.1 += by;
            }
        }
        if let Some((seat, _)) = totals.iter().find(|t| t.1 <= 0) {
            return Some(*seat);
        }
    }
    None
}

// ------------------------------------------------------------- the game's end

/// The owner's own example: eighteen Dingus Egg triggers, nine for each side,
/// three of them enough to end either player.
#[test]
fn eighteen_dingus_eggs_hit_the_opponent_first() {
    let mut waiting = Vec::new();
    for _ in 0..9 {
        waiting.push(real("Dingus Egg", Some(ME)));
        waiting.push(real("Dingus Egg", Some(THEM)));
    }
    let table = table(6, 6);
    let suggestion = suggest(&waiting, &table, &TriggerMemory::default());
    assert_eq!(suggestion.reason, Reason::Wins);
    assert_eq!(
        first_to_fall(&waiting, &suggestion.order, &table),
        Some(THEM)
    );
    assert_eq!(suggestion.order.len(), 18);
}

/// With life to spare the order decides nothing, and it is good play that
/// still hits the opponent first.
#[test]
fn with_life_to_spare_the_opponent_is_still_hit_first_and_it_says_why() {
    let waiting = [real("Dingus Egg", Some(ME)), real("Dingus Egg", Some(THEM))];
    let suggestion = suggest(&waiting, &table(20, 20), &TriggerMemory::default());
    assert_eq!(suggestion.order, vec![1, 0]);
    assert_eq!(suggestion.reason, Reason::OpponentFirst);
}

#[test]
fn life_gained_first_keeps_the_player_in_the_game() {
    let waiting = [trigger(1, life(ME, -3)), trigger(2, life(ME, 3))];
    let suggestion = suggest(&waiting, &table(2, 20), &TriggerMemory::default());
    assert_eq!(suggestion.order, vec![1, 0]);
    assert_eq!(suggestion.reason, Reason::Survives);
}

#[test]
fn an_order_that_cannot_save_the_player_claims_nothing() {
    let waiting = [trigger(1, life(ME, -5)), trigger(2, life(ME, 1))];
    let suggestion = suggest(&waiting, &table(2, 20), &TriggerMemory::default());
    assert_ne!(suggestion.reason, Reason::Survives);
    assert_eq!(suggestion.reason, Reason::GainFirst);
}

/// One opponent of two falling is no win: the game goes on, and a win is
/// what the line would have said.
#[test]
fn one_of_two_opponents_falling_is_not_called_a_win() {
    let mut table = table(20, 2);
    table.players.push(standing(THIRD, 20));
    let waiting = [trigger(1, life(ME, -3)), trigger(2, life(THEM, -2))];
    let suggestion = suggest(&waiting, &table, &TriggerMemory::default());
    assert_eq!(suggestion.order, vec![1, 0]);
    assert_eq!(suggestion.reason, Reason::OpponentFirst);
}

/// Every opponent falling is, however many there are.
#[test]
fn the_last_of_two_opponents_falling_first_wins() {
    let mut table = table(1, 2);
    table.players.push(standing(THIRD, 1));
    let waiting = [
        trigger(1, life(ME, -1)),
        trigger(2, life(THEM, -2)),
        trigger(3, life(THIRD, -1)),
    ];
    let suggestion = suggest(&waiting, &table, &TriggerMemory::default());
    assert_eq!(suggestion.reason, Reason::Wins);
    assert_eq!(suggestion.order[2], 0, "the player's own comes last");
}

/// Poison and a library drawn past its end end a game as life does.
#[test]
fn poison_and_an_empty_library_end_the_game_too() {
    let poison = Facts {
        changes: vec![Change {
            who: THEM,
            what: Total::Poison(1),
        }],
        ..Facts::default()
    };
    let mut near = table(1, 20);
    near.players[1].poison = 9;
    let waiting = [trigger(1, life(ME, -1)), trigger(2, poison)];
    let suggestion = suggest(&waiting, &near, &TriggerMemory::default());
    assert_eq!(
        (suggestion.order.clone(), suggestion.reason),
        (vec![1, 0], Reason::Wins)
    );

    let draw = Facts {
        changes: vec![Change {
            who: THEM,
            what: Total::Draws(2),
        }],
        ..Facts::default()
    };
    let mut near = table(1, 20);
    near.players[1].library = 1;
    let waiting = [trigger(1, life(ME, -1)), trigger(2, draw)];
    let suggestion = suggest(&waiting, &near, &TriggerMemory::default());
    assert_eq!(
        (suggestion.order, suggestion.reason),
        (vec![1, 0], Reason::Wins)
    );
}

/// A win is never undone by a habit, and the sheet asks instead of going on
/// by itself, because the habit was not what the player would have got.
#[test]
fn a_win_outweighs_what_the_player_taught_and_is_asked_about() {
    let mine = trigger(1, life(ME, -1));
    let theirs = trigger(2, life(THEM, -1));
    let mut memory = TriggerMemory::default();
    memory.learn(&[Some(key(2)), Some(key(1))], &[Some(key(1)), Some(key(2))]);
    memory.accept_known = true;
    let waiting = [mine, theirs];
    let suggestion = suggest(&waiting, &table(1, 1), &memory);
    assert_eq!(suggestion.order, vec![1, 0]);
    assert_eq!(suggestion.reason, Reason::Wins);
    assert!(!suggestion.goes_unasked(&memory));
}

/// Past what can be proven it says nothing about the game's end, and the
/// question still gets an answer.
#[test]
fn too_many_different_triggers_to_prove_anything_still_get_an_order() {
    let waiting: Vec<Waiting> = (0..40_i32)
        .map(|n| trigger(u32::try_from(n).unwrap(), life(THEM, -1 - n)))
        .collect();
    let suggestion = suggest(&waiting, &table(20, 5), &TriggerMemory::default());
    assert_ne!(suggestion.reason, Reason::Wins);
    assert_eq!(suggestion.order.len(), 40);
}

// ------------------------------------------------------------------ the memory

#[test]
fn what_the_player_taught_decides_and_is_marked() {
    let waiting = [trigger(1, Facts::default()), trigger(2, Facts::default())];
    let mut memory = TriggerMemory::default();
    assert_eq!(
        suggest(&waiting, &table(20, 20), &memory).order,
        vec![0, 1],
        "with nothing to go on, as they came"
    );
    memory.learn(&[Some(key(1)), Some(key(2))], &[Some(key(2)), Some(key(1))]);
    let suggestion = suggest(&waiting, &table(20, 20), &memory);
    assert_eq!(suggestion.order, vec![1, 0]);
    assert_eq!(suggestion.reason, Reason::Remembered);
    assert_eq!(suggestion.remembered, vec![true, true]);
}

/// Teaching outweighs good play: the player may know better.
#[test]
fn what_the_player_taught_outweighs_good_play() {
    let waiting = [
        trigger(1, doing(Deeds::HEALS_ME)),
        trigger(2, doing(Deeds::HURTS_ME)),
    ];
    assert_eq!(
        suggest(&waiting, &table(20, 20), &TriggerMemory::default()).order,
        vec![0, 1]
    );
    let mut memory = TriggerMemory::default();
    memory.learn(&[Some(key(1)), Some(key(2))], &[Some(key(2)), Some(key(1))]);
    assert_eq!(suggest(&waiting, &table(20, 20), &memory).order, vec![1, 0]);
}

/// A new case with one pair known: the pair holds, and good play fills in
/// the rest, marked apart.
#[test]
fn a_case_partly_known_is_put_together_from_both() {
    let waiting = [
        trigger(1, Facts::default()),
        trigger(2, Facts::default()),
        trigger(3, doing(Deeds::HURTS_ME)),
        trigger(4, doing(Deeds::HEALS_ME)),
    ];
    let mut memory = TriggerMemory::default();
    memory.learn(&[Some(key(1)), Some(key(2))], &[Some(key(2)), Some(key(1))]);
    let suggestion = suggest(&waiting, &table(20, 20), &memory);
    let at = |index: usize| suggestion.order.iter().position(|i| *i == index).unwrap();
    assert!(at(1) < at(0), "the pair the player taught");
    assert!(at(3) < at(2), "and good play for the rest");
    assert_eq!(suggestion.remembered, vec![true, true, false, false]);
    assert!(!suggestion.all_known);
}

/// A before B, B before C, and later C before A: no order holds all three.
/// The newest teaching stands, the oldest gives way, and nothing hangs.
#[test]
fn a_circle_of_teaching_gives_way_at_its_oldest_pair() {
    let waiting = [
        trigger(1, Facts::default()),
        trigger(2, Facts::default()),
        trigger(3, Facts::default()),
    ];
    let mut memory = TriggerMemory::default();
    let one = Some(key(1));
    let two = Some(key(2));
    let three = Some(key(3));
    memory.learn(&[two, one], &[one, two]);
    memory.learn(&[three, two], &[two, three]);
    memory.learn(&[one, three], &[three, one]);
    assert_eq!(memory.len(), 3);
    let suggestion = suggest(&waiting, &table(20, 20), &memory);
    assert_eq!(suggestion.order, vec![1, 2, 0]);
    assert!(!suggestion.all_known, "one of the three did not hold");
}

/// Copies of one ability share the teaching: the key is the ability.
#[test]
fn the_teaching_holds_for_every_copy_and_across_games() {
    let mut memory = TriggerMemory::default();
    memory.learn(&[Some(key(1)), Some(key(2))], &[Some(key(2)), Some(key(1))]);
    let mut first = trigger(1, Facts::default());
    first.source = Some(ObjectId::new(11, 0));
    let mut second = trigger(1, Facts::default());
    second.source = Some(ObjectId::new(12, 0));
    let waiting = [first, second, trigger(2, Facts::default())];
    assert_eq!(
        suggest(&waiting, &table(20, 20), &memory).order,
        vec![2, 0, 1]
    );
}

#[test]
fn a_question_answered_whole_goes_unasked_only_when_the_player_allowed_it() {
    let waiting = [trigger(1, Facts::default()), trigger(2, Facts::default())];
    let mut memory = TriggerMemory::default();
    memory.learn(&[Some(key(1)), Some(key(2))], &[Some(key(2)), Some(key(1))]);
    let suggestion = suggest(&waiting, &table(20, 20), &memory);
    assert!(suggestion.all_known);
    assert!(!suggestion.goes_unasked(&memory), "off unless turned on");
    memory.accept_known = true;
    assert!(suggestion.goes_unasked(&memory));
}

/// Copies that differ are no pair the teaching knows, and an ability nothing
/// names can never be known.
#[test]
fn what_the_teaching_cannot_name_is_always_asked() {
    let mut memory = TriggerMemory::default();
    memory.accept_known = true;
    let mut other = trigger(1, Facts::default());
    other.twin = 9;
    let waiting = [other, trigger(1, Facts::default())];
    assert!(!suggest(&waiting, &table(20, 20), &memory).goes_unasked(&memory));

    let mut nameless = trigger(1, Facts::default());
    nameless.key = None;
    let waiting = [nameless, trigger(2, Facts::default())];
    assert!(!suggest(&waiting, &table(20, 20), &memory).goes_unasked(&memory));
}

// ------------------------------------------------------------------- good play

/// One rule at a time: `first` before `then`, in whichever order they came,
/// and the sheet names that rule.
fn expect(first: Waiting, then: Waiting, reason: Reason) {
    for (waiting, order) in [
        ([first.clone(), then.clone()], vec![0, 1]),
        ([then, first], vec![1, 0]),
    ] {
        let suggestion = suggest(&waiting, &table(20, 20), &TriggerMemory::default());
        assert_eq!(suggestion.order, order, "{reason:?}");
        assert_eq!(suggestion.reason, reason);
    }
}

fn makes(stuff: Stuff) -> Facts {
    Facts {
        makes: stuff,
        ..Facts::default()
    }
}

fn reads(stuff: Stuff) -> Facts {
    Facts {
        reads: stuff,
        ..Facts::default()
    }
}

#[test]
fn what_gets_counted_comes_first() {
    for stuff in [
        Stuff::COUNTERS,
        Stuff::BOARD,
        Stuff::GRAVEYARD,
        Stuff::MANA,
        Stuff::POWER,
    ] {
        expect(
            trigger(1, makes(stuff)),
            trigger(2, reads(stuff)),
            Reason::CountedLater,
        );
    }
}

/// Counters on one permanent mean nothing to another's count of its own.
#[test]
fn a_counter_matters_to_its_own_permanent_only() {
    let mut adds = trigger(1, makes(Stuff::OWN_COUNTERS));
    let mut counts = trigger(2, reads(Stuff::OWN_COUNTERS));
    adds.source = Some(ObjectId::new(1, 0));
    counts.source = Some(ObjectId::new(2, 0));
    let waiting = [counts.clone(), adds.clone()];
    assert_eq!(
        suggest(&waiting, &table(20, 20), &TriggerMemory::default()).order,
        vec![0, 1],
        "two permanents: nothing to say"
    );
    counts.source = adds.source;
    expect(adds, counts, Reason::CountedLater);
}

#[test]
fn a_shuffle_comes_before_looking_and_looking_before_drawing() {
    expect(
        trigger(1, doing(Deeds::SHUFFLES)),
        trigger(2, doing(Deeds::LOOKS)),
        Reason::ShuffleFirst,
    );
    expect(
        trigger(1, doing(Deeds::LOOKS)),
        trigger(2, doing(Deeds::DRAWS)),
        Reason::LookFirst,
    );
}

#[test]
fn seeing_more_comes_before_deciding() {
    for deed in [Deeds::DRAWS, Deeds::LOOKS, Deeds::SEARCHES] {
        expect(
            trigger(1, doing(deed)),
            trigger(2, doing(Deeds::ASKS)),
            Reason::InformFirst,
        );
    }
}

#[test]
fn a_trigger_that_takes_its_source_away_waits_for_the_sources_others() {
    let source = Some(ObjectId::new(7, 0));
    let mut stays = trigger(1, Facts::default());
    let mut leaves = trigger(2, doing(Deeds::LEAVES));
    stays.source = source;
    leaves.source = source;
    expect(stays, leaves, Reason::SourceStays);
}

#[test]
fn removal_comes_before_a_boost() {
    expect(
        trigger(1, doing(Deeds::REMOVES)),
        trigger(2, doing(Deeds::PUMPS)),
        Reason::RemovalFirst,
    );
}

#[test]
fn the_opponent_is_hit_first_and_life_is_gained_first() {
    expect(
        trigger(1, doing(Deeds::HURTS_OPPONENT)),
        trigger(2, doing(Deeds::HURTS_ME)),
        Reason::OpponentFirst,
    );
    expect(
        trigger(1, doing(Deeds::HEALS_ME)),
        trigger(2, doing(Deeds::HURTS_ME)),
        Reason::GainFirst,
    );
}

/// What depends on what outweighs advantage: when two rules pull apart, the
/// heavier one holds and the sheet names it.
#[test]
fn a_heavier_rule_is_not_undone_by_a_lighter_one() {
    let mut pays = doing(Deeds::ASKS);
    pays.makes = Stuff::COUNTERS;
    let mut informs = doing(Deeds::DRAWS);
    informs.reads = Stuff::COUNTERS;
    expect(trigger(1, pays), trigger(2, informs), Reason::CountedLater);
}

/// Two triggers with nothing between them stay as they came, and say so.
#[test]
fn nothing_to_go_on_says_so_and_moves_nothing() {
    let waiting = [trigger(1, Facts::default()), trigger(2, Facts::default())];
    let suggestion = suggest(&waiting, &table(20, 20), &TriggerMemory::default());
    assert_eq!(suggestion.order, vec![0, 1]);
    assert_eq!(suggestion.reason, Reason::NoOpinion);
}

/// Twins need no pair of their own: nothing in the plan moves one past the
/// other, and the teaching does not ask to know them.
#[test]
fn twins_stay_as_they_came_and_need_no_teaching() {
    let waiting = [
        real("Dingus Egg", Some(THEM)),
        real("Dingus Egg", Some(THEM)),
    ];
    let memory = TriggerMemory::default();
    let suggestion = suggest(&waiting, &table(20, 20), &memory);
    assert_eq!(suggestion.order, vec![0, 1]);
    assert!(!suggestion.all_known, "there is no pair to know");
}

/// Whatever is asked, an order comes back holding every trigger once, and
/// the same question gets the same answer.
#[test]
fn every_question_gets_one_whole_order_and_always_the_same_one() {
    let deeds = [
        Deeds::DRAWS,
        Deeds::LOOKS,
        Deeds::SHUFFLES,
        Deeds::ASKS,
        Deeds::REMOVES,
        Deeds::PUMPS,
        Deeds::HURTS_ME,
        Deeds::HEALS_ME,
        Deeds::HURTS_OPPONENT,
    ];
    let mut seed = 0x2545_f491_u32;
    let mut next = move |bound: u32| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 16) % bound
    };
    for _ in 0..300 {
        let waiting: Vec<Waiting> = (0..next(24))
            .map(|_| {
                let mut facts = doing(deeds[next(9) as usize]);
                if next(3) == 0 {
                    facts.changes.push(Change {
                        who: if next(2) == 0 { ME } else { THEM },
                        what: Total::Life(i32::try_from(next(5)).unwrap() - 3),
                    });
                }
                trigger(next(8), facts)
            })
            .collect();
        let mut memory = TriggerMemory::default();
        for _ in 0..next(12) {
            let (a, b) = (Some(key(next(8))), Some(key(next(8))));
            memory.learn(&[a, b], &[b, a]);
        }
        let table = table(i32::try_from(next(9)).unwrap() + 1, 7);
        let suggestion = suggest(&waiting, &table, &memory);
        assert_eq!(suggestion, suggest(&waiting, &table, &memory));
        let mut sorted = suggestion.order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..waiting.len()).collect::<Vec<_>>());
    }
}

// ------------------------------------------------------------- reading the cards

/// What a real card's script says, for the verbs a rule stands on.
#[test]
fn real_cards_read_as_printed() {
    let egg = real("Dingus Egg", Some(THEM));
    assert_eq!(
        egg.facts.changes,
        vec![Change {
            who: THEM,
            what: Total::Life(-2)
        }]
    );
    assert!(egg.facts.deeds.has(Deeds::HURTS_OPPONENT));
    assert!(
        real("Dingus Egg", Some(ME))
            .facts
            .deeds
            .has(Deeds::HURTS_ME)
    );
    assert!(
        real("Dingus Egg", None).facts.changes.is_empty(),
        "whose land it was is not known: nothing is certain"
    );
    let warden = real("Soul Warden", None);
    assert!(warden.facts.deeds.has(Deeds::HEALS_ME));
    assert_eq!(
        warden.facts.changes,
        vec![Change {
            who: ME,
            what: Total::Life(1)
        }]
    );
    assert!(warden.facts.makes.meets(Stuff::LIFE, Stuff::ANYWHERE));
}

/// The teaching travels in the account's preferences, and a blob written
/// before it existed still opens.
#[test]
fn the_teaching_rides_with_the_account() {
    use crate::prefs::Preferences;
    let mut prefs = Preferences::default();
    prefs
        .trigger_order
        .learn(&[Some(key(1)), Some(key(2))], &[Some(key(2)), Some(key(1))]);
    let written = serde_json::to_string(&prefs).unwrap();
    let read: Preferences = serde_json::from_str(&written).unwrap();
    assert_eq!(read.trigger_order, prefs.trigger_order);
    let old: Preferences = serde_json::from_str("{}").unwrap();
    assert!(old.trigger_order.is_empty());
    assert!(!old.trigger_order.accept_known, "off unless turned on");
}

/// A token with a Soul Warden on the table is a life gain: placed like one,
/// ahead of what costs the player life, though the token itself says nothing.
#[test]
fn a_trigger_that_sets_others_off_stands_where_they_belong() {
    let opponents = [THEM];
    let reading = Reading {
        me: ME,
        opponents: &opponents,
        event_player: None,
        event_amount: None,
    };
    let (_, warden) = printed("Soul Warden");
    let followers: Vec<_> = facts::follower(warden, &reading).into_iter().collect();
    assert_eq!(followers.len(), 1, "Soul Warden watches what enters");
    let token = makes(Stuff::BOARD);
    let hurts = trigger(2, doing(Deeds::HURTS_ME));
    let waiting = [hurts.clone(), trigger(1, token.clone())];
    assert_eq!(
        suggest(&waiting, &table(20, 20), &TriggerMemory::default()).order,
        vec![0, 1],
        "alone, a token has nothing to do with life"
    );
    let waiting = [hurts, trigger(1, token.with_followers(&followers))];
    let suggestion = suggest(&waiting, &table(20, 20), &TriggerMemory::default());
    assert_eq!(suggestion.order, vec![1, 0]);
    assert_eq!(suggestion.reason, Reason::GainFirst);
}

/// One follower waking another is followed to the end, and a circle of them
/// ends too.
#[test]
fn followers_are_followed_and_a_circle_of_them_ends() {
    let opponents = [THEM];
    let reading = Reading {
        me: ME,
        opponents: &opponents,
        event_player: None,
        event_amount: None,
    };
    let (_, warden) = printed("Soul Warden");
    let follower = facts::follower(warden, &reading).expect("a follower");
    let read = makes(Stuff::BOARD).with_followers(&[follower.clone(), follower]);
    assert!(read.deeds.has(Deeds::HEALS_ME));
    assert!(
        read.changes.is_empty(),
        "nothing a follower does is certain"
    );
}

/// How much of the pool's triggers the reading has something to say about:
/// the number a new verb going unread would lower.
#[test]
fn the_reading_keeps_up_with_the_pool() {
    let opponents = [THEM];
    let reading = Reading {
        me: ME,
        opponents: &opponents,
        event_player: Some(THEM),
        event_amount: Some(2),
    };
    let (mut triggers, mut read) = (0_u32, 0_u32);
    for card in baylee_cards::all() {
        for ability in card.abilities {
            if matches!(
                ability,
                AbilityDef::Triggered { .. } | AbilityDef::ModalTriggered { .. }
            ) {
                triggers += 1;
                read += u32::from(facts::of_ability(ability, &reading).known());
            }
        }
    }
    assert!(triggers > 2_000, "the pool's triggers: {triggers}");
    assert!(
        read * 100 >= triggers * MEASURED_PERCENT,
        "{read} of {triggers} read"
    );
}

/// Measured when the reading was written; raise it as verbs are added.
const MEASURED_PERCENT: u32 = 84;
