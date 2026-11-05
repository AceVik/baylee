//! Complete life-gain replacements (CR 614.5, 616.1): Sulfuric Vortex's
//! no-life event, Lich's draws, their affected player's choice, and the queue
//! that keeps a later gain behind that choice.

use super::*;
use crate::choice::ChoicePrompt;
use baylee_cards_dsl::{Amount, Filter, Modifier, PlayerRel};

const NO_LIFE: u32 = 7720;
const DRAW_LIFE: u32 = 7721;
const GIVER: u32 = 7722;

static NO_LIFE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::Any,
    Modifier::LifeGainNoLifeInstead {
        who: PlayerRel::EachPlayer
    }
)];
static DRAW_LIFE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::Any,
    Modifier::LifeGainDrawsInstead {
        who: PlayerRel::You
    }
)];
static GIVER_ABILITIES: &[AbilityDef] = &[
    free(&[Effect::gain_life(2)], None),
    free(
        &[Effect::GainLifeFor {
            amount: Amount::Fixed(4),
            who: PlayerRel::EachPlayer,
        }],
        None,
    ),
];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            NO_LIFE,
            face("No-life probe", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            NO_LIFE_ABILITIES,
        ),
        card(
            DRAW_LIFE,
            face("Draw-life probe", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DRAW_LIFE_ABILITIES,
        ),
        card(
            GIVER,
            face("Life-gain probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            GIVER_ABILITIES,
        ),
    ]
}

fn game(board: &[u32]) -> Bench {
    let mut with_giver = board.to_vec();
    with_giver.push(GIVER);
    let mut engine = bench(
        7720,
        cards(),
        [Seat::with(&with_giver).at(10), Seat::default().at(10)],
    );
    to_main(&mut engine, me());
    engine
}

fn gain_until_choice(engine: &mut Bench, ability: u32) {
    let giver = the(engine, ZoneLocation::Battlefield, GIVER);
    activate(engine, me(), giver, ability);
    walk_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LifeGainReplacement,
                ..
            }
        )
    });
}

fn choose_source(engine: &mut Bench, index: u32) {
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected a life-gain replacement: {:?}", engine.pending())
    };
    let source = the(engine, ZoneLocation::Battlefield, index);
    assert!(options.contains(&source), "missing {index}: {options:?}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .unwrap();
    settle(engine, me());
}

fn hand_size(engine: &Bench, player: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(player)).len()
}

#[test]
fn no_life_alone_replaces_the_gain_without_an_event_or_tally() {
    let mut engine = game(&[NO_LIFE]);
    let since = engine.journal().last_seq();
    let giver = the(&engine, ZoneLocation::Battlefield, GIVER);
    activate(&mut engine, me(), giver, 0);
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 10);
    assert_eq!(hand_size(&engine, me()), 0);
    assert_eq!(engine.state().per_turn.life_gained_amount[0], 0);
    let mut new_entries = engine
        .journal()
        .entries()
        .iter()
        .skip(usize::try_from(since).expect("journal length fits usize"));
    assert!(!new_entries.any(|entry| {
        matches!(entry.event, GameEvent::LifeChanged { player, .. } if player == me())
    }));
}

#[test]
fn the_affected_player_may_choose_no_life_over_lich_draws() {
    let mut engine = game(&[NO_LIFE, DRAW_LIFE]);
    gain_until_choice(&mut engine, 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2);
    assert_eq!(engine.state().life_gains_to_offer.len(), 1);

    choose_source(&mut engine, NO_LIFE);
    assert_eq!(life(&engine, me()), 10);
    assert_eq!(hand_size(&engine, me()), 0);
    assert_eq!(engine.state().per_turn.life_gained_amount[0], 0);
    assert!(engine.state().life_gains_to_offer.is_empty());
}

#[test]
fn the_affected_player_may_choose_lich_draws_over_no_life() {
    let mut engine = game(&[NO_LIFE, DRAW_LIFE]);
    gain_until_choice(&mut engine, 0);
    choose_source(&mut engine, DRAW_LIFE);

    assert_eq!(life(&engine, me()), 10);
    assert_eq!(hand_size(&engine, me()), 2);
    assert_eq!(engine.state().per_turn.life_gained_amount[0], 0);
    assert!(engine.state().life_gains_to_offer.is_empty());
}

#[test]
fn several_identical_replacements_still_replace_only_once_without_a_question() {
    let mut engine = game(&[NO_LIFE, NO_LIFE]);
    let giver = the(&engine, ZoneLocation::Battlefield, GIVER);
    activate(&mut engine, me(), giver, 0);
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 10);
    assert_eq!(hand_size(&engine, me()), 0);
    assert!(engine.state().life_gains_to_offer.is_empty());
}

#[test]
fn a_later_players_gain_waits_behind_the_front_choice() {
    let mut engine = game(&[NO_LIFE, DRAW_LIFE]);
    gain_until_choice(&mut engine, 1);
    let waiting: Vec<_> = engine
        .state()
        .life_gains_to_offer
        .iter()
        .map(|gain| (gain.player, gain.amount))
        .collect();
    assert_eq!(waiting, vec![(me(), 4), (them(), 4)]);
    assert_eq!(life(&engine, them()), 10);

    choose_source(&mut engine, DRAW_LIFE);
    assert_eq!(hand_size(&engine, me()), 4);
    assert_eq!(hand_size(&engine, them()), 0);
    assert_eq!(life(&engine, me()), 10);
    assert_eq!(life(&engine, them()), 10);
    assert!(engine.state().life_gains_to_offer.is_empty());
}

/// A player who leaves while choosing makes no choice (CR 800.4f), and the gain
/// that was theirs goes with them; the seat behind them is still settled.
#[test]
fn leaving_during_the_choice_drops_that_gain_and_settles_the_next() {
    let third = PlayerId::new(2);
    let mut engine = bench(
        7721,
        cards(),
        [
            Seat::with(&[GIVER]).at(10),
            Seat::with(&[NO_LIFE, DRAW_LIFE]).at(10),
            Seat::default().at(10),
        ],
    );
    to_main(&mut engine, me());
    let giver = the(&engine, ZoneLocation::Battlefield, GIVER);
    activate(&mut engine, me(), giver, 1);
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LifeGainReplacement,
                ..
            }
        )
    });
    assert_eq!(engine.pending().asked(), Some(them()));
    assert_eq!(
        engine
            .state()
            .life_gains_to_offer
            .iter()
            .map(|gain| gain.player)
            .collect::<Vec<_>>(),
        vec![them(), third]
    );

    engine.apply(them(), PlayerAction::Concede).unwrap();

    assert!(
        engine.state().life_gains_to_offer.is_empty(),
        "the leaver's gain went with them and the one behind it was settled"
    );
    assert_eq!(hand_size(&engine, them()), 0, "no draw was chosen");
    assert_ne!(engine.pending().asked(), Some(them()));
}
