//! `cards/creatures/mv_3/mari_the_killing_quill.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::{index, subtypes};
use baylee_engine::event::Cause;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mari() -> CardIndex {
    card_index("5f1fdc23-9af0-41a1-aeba-7288f9642734")
}

fn move_to(e: &mut Engine<RegistryLookup>, card: ObjectId, to: ZoneLocation) {
    e.dev_state_mut(P0)
        .expect("the harness may prepare the public zone")
        .move_object(card, to, ZonePosition::Top, Cause::Effect)
        .expect("the card moves");
}

fn collect(e: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    let player = *player;
    e.apply(player, PlayerAction::PassPriority).unwrap();
}

fn treasures(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    tokens_of(e, seat)
        .into_iter()
        .filter(|id| {
            e.state().object(*id).is_some_and(|object| {
                object
                    .characteristics()
                    .subtypes
                    .contains(subtypes::artifact::TREASURE)
            })
        })
        .count()
}

#[test]
fn mari_exiles_only_an_opponents_dead_creature_with_one_hit_counter() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &[mari(), steadfast_guard()])
        .battlefield(1, &[giant_spider()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let ours = on_battlefield(&e, P0, steadfast_guard()).unwrap();
    let theirs = on_battlefield(&e, P1, giant_spider()).unwrap();

    kill(&mut e, ours);
    assert_eq!(e.state().object(ours).unwrap().zone, Zone::Graveyard);
    assert_eq!(counters_on(&e, ours, counters::HIT), 0);

    kill(&mut e, theirs);
    assert_eq!(e.state().object(theirs).unwrap().zone, Zone::Exile);
    assert_eq!(counters_on(&e, theirs, counters::HIT), 1);
    assert_eq!(e.state().object(theirs).unwrap().owner, P1);
}

#[test]
fn mari_cannot_reach_a_card_that_returned_before_her_trigger_resolved() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &[mari()])
        .battlefield(1, &[steadfast_guard()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let victim = on_battlefield(&e, P1, steadfast_guard()).unwrap();

    move_to(&mut e, victim, ZoneLocation::Graveyard(P1));
    collect(&mut e);
    assert!(!e.state().zones.stack_is_empty(), "Mari triggered");
    move_to(&mut e, victim, ZoneLocation::Battlefield);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(victim).unwrap().zone, Zone::Battlefield);
    assert_eq!(counters_on(&e, victim, counters::HIT), 0);
    assert!(e.state().zones.list(ZoneLocation::Exile(P1)).is_empty());
}

#[test]
fn mari_grants_deathtouch_to_exactly_assassins_mercenaries_and_rogues() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mari(),
                index::GOBLIN_TURNCOAT,
                index::BLACKBLOOM_ROGUE,
                steadfast_guard(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    for card in [mari(), index::GOBLIN_TURNCOAT, index::BLACKBLOOM_ROGUE] {
        let creature = on_battlefield(&e, P0, card).unwrap();
        assert!(keywords(&e, creature).contains(KeywordSet::DEATHTOUCH));
    }
    let other = on_battlefield(&e, P0, steadfast_guard()).unwrap();
    assert!(!keywords(&e, other).contains(KeywordSet::DEATHTOUCH));
}

#[test]
fn mari_plays_granted_deathtouch_in_combat_and_marks_the_dead_blocker() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &[mari()])
        .battlefield(1, &[giant_spider()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let attacker = on_battlefield(&e, P0, mari()).unwrap();
    let blocker = on_battlefield(&e, P1, giant_spider()).unwrap();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(e.state().object(attacker).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(blocker).unwrap().zone, Zone::Exile);
    assert_eq!(counters_on(&e, blocker, counters::HIT), 1);
}

fn reward_board(attacker_card: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let attackers = if attacker_card == mari() {
        vec![mari()]
    } else {
        vec![mari(), attacker_card]
    };
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &attackers)
        .battlefield(1, &[steadfast_guard()])
        .hand(0, &[forest(), llanowar_elves()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let attacker = on_battlefield(&e, P0, attacker_card).unwrap();
    let victim = on_battlefield(&e, P1, steadfast_guard()).unwrap();
    kill(&mut e, victim);
    assert_eq!(counters_on(&e, victim, counters::HIT), 1);

    // A second counter proves exactly one is removed. Two other exiled cards
    // prove that owner and counter eligibility are independent filters.
    let state = e.dev_state_mut(P0).unwrap();
    baylee_engine::replacement::put_counters(state, victim, counters::HIT, 1);
    let own_marked = hand_to(&mut e, P0, llanowar_elves(), ZoneLocation::Exile(P0));
    let state = e.dev_state_mut(P0).unwrap();
    baylee_engine::replacement::put_counters(state, own_marked, counters::HIT, 1);
    hand_to(&mut e, P1, llanowar_elves(), ZoneLocation::Exile(P1));
    hand_to_library_top(&mut e, P0, forest());
    (e, attacker, victim)
}

#[track_caller]
fn connect(e: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player: P0,
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
}

#[test]
fn each_printed_type_plays_the_hit_payment_draw_and_two_treasures() {
    for attacker_card in [mari(), index::GOBLIN_TURNCOAT, index::BLACKBLOOM_ROGUE] {
        let (mut e, attacker, hit_card) = reward_board(attacker_card);
        let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
        let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
        assert_eq!(treasures(&e, P0), 0);
        connect(&mut e, attacker);
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();

        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending().clone()
        else {
            panic!("the hit card is chosen: {:?}", e.pending())
        };
        assert_eq!(options, vec![hit_card], "only their marked card");
        assert_eq!((min, max), (1, 1));
        e.apply(P0, PlayerAction::ChooseObjects { objects: options })
            .unwrap();
        pass_until(&mut e, stack_is_empty);

        assert_eq!(counters_on(&e, hit_card, counters::HIT), 1);
        assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand + 1);
        assert_eq!(
            e.state().zones.list(ZoneLocation::Library(P0)).len(),
            library - 1
        );
        assert!(
            in_hand(&e, P0, forest()).is_some(),
            "the top card was drawn"
        );
        assert_eq!(treasures(&e, P0), 2);
    }
}

#[test]
fn mari_declining_the_hit_payment_changes_no_counter_hand_or_tokens() {
    let (mut e, attacker, hit_card) = reward_board(mari());
    let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    connect(&mut e, attacker);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(counters_on(&e, hit_card, counters::HIT), 2);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library
    );
    assert_eq!(treasures(&e, P0), 0);
}

#[test]
fn mari_without_a_legal_hit_card_does_not_offer_the_reward() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &[mari()])
        .hand(0, &[llanowar_elves()])
        .hand(1, &[steadfast_guard()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let attacker = on_battlefield(&e, P0, mari()).unwrap();
    let own_marked = hand_to(&mut e, P0, llanowar_elves(), ZoneLocation::Exile(P0));
    let state = e.dev_state_mut(P0).unwrap();
    baylee_engine::replacement::put_counters(state, own_marked, counters::HIT, 1);
    hand_to(&mut e, P1, steadfast_guard(), ZoneLocation::Exile(P1));
    let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(counters_on(&e, own_marked, counters::HIT), 1);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library
    );
    assert_eq!(treasures(&e, P0), 0);
}
