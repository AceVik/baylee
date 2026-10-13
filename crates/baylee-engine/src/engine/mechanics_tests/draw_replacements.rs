//! Replacements of a card draw (K12 of the partials lane), one rule per
//! test, played by cards nobody printed:
//!
//! - `ReplacementRule::Dredge` (Golgari Thug, Dakmor Salvage): CR 702.52a-b.
//! - `ReplacementRule::DiscardInsteadOfLaterDraws` (Chains of
//!   Mephistopheles): every draw but the first of a draw step.
//! - `Effect::NextDrawLooksAtTop` (Aladdin's Lamp).
//! - The drawing player's choice among several (CR 616.1), and the draw a
//!   replacement leaves being asked about again (CR 616.1f, 616.2).
//! - "Draw a card and reveal it" after a draw that had to wait (CR 614.11b).
//!
//! Every library here is Forests, so a card is told apart by its id and
//! where it ends, never by its name.

use super::*;
use crate::choice::ChoicePrompt;
use crate::event::GameEvent;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, ReplacementRule};

const DREDGER: u32 = 7700;
const CHAINS: u32 = 7701;
const LAMP: u32 = 7702;
const DRAWER: u32 = 7703;
const MILLER: u32 = 7704;
const SEER: u32 = 7705;
const BAUBLE: u32 = 7706;

/// "Dredge 2", and "{0}: Sacrifice this" to put it where dredge works.
static DREDGER_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Replacement(ReplacementRule::Dredge(2)),
    free(&[Effect::SacrificeSelf], None),
];

static CHAINS_ABILITIES: &[AbilityDef] = &[AbilityDef::Replacement(
    ReplacementRule::DiscardInsteadOfLaterDraws,
)];

/// "{0}: The next time you would draw a card this turn, instead look at the
/// top three cards of your library, put all but one of them on the bottom
/// of your library in a random order, then draw a card."
static LAMP_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::NextDrawLooksAtTop {
        count: Amount::Fixed(3),
    }],
    None,
)];

/// 0: "{0}: Draw a card"; 1: "{0}: Draw two cards".
static DRAWER_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        None,
    ),
    free(
        &[Effect::DrawCards {
            amount: Amount::Fixed(2),
        }],
        None,
    ),
];

/// "{0}: Mill fifty-seven cards", to bring a library down.
static MILLER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::Mill {
        amount: Amount::Fixed(57),
        target: PlayerRel::You,
    }],
    None,
)];

/// Sindbad's sentence: "{0}: Draw a card and reveal it. If it isn't a land
/// card, discard it."
static SEER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::DrawRevealDiscardUnless {
        keep: &Filter::LAND,
    }],
    None,
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            DREDGER,
            creature_face("Dredger", "{1}{B}", 1, 1),
            KeywordSet::EMPTY,
            DREDGER_ABILITIES,
        ),
        card(
            CHAINS,
            face("Chains", "{1}{B}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            CHAINS_ABILITIES,
        ),
        card(
            LAMP,
            face("Lamp", "{10}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            LAMP_ABILITIES,
        ),
        card(
            DRAWER,
            face("Drawer", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DRAWER_ABILITIES,
        ),
        card(
            MILLER,
            face("Miller", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            MILLER_ABILITIES,
        ),
        card(
            SEER,
            face("Seer", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SEER_ABILITIES,
        ),
        card(
            BAUBLE,
            face("Bauble", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
    ]
}

fn game(mine: Seat) -> Bench {
    let mut engine = bench(7700, cards(), [mine, Seat::default()]);
    to_main(&mut engine, me());
    engine
}

fn library(engine: &Bench, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

fn hand(engine: &Bench, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

fn graveyard(engine: &Bench, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .clone()
}

fn draws(engine: &Bench, seat: PlayerId) -> u32 {
    engine.state().per_turn.draws[usize::from(seat.get())]
}

/// The card question on the table: its options, bounds and prompt.
#[track_caller]
fn asked(engine: &Bench) -> (Vec<ObjectId>, u8, u8, ChoicePrompt) {
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            ..
        } => {
            assert_eq!(*player, me(), "the drawing player is asked");
            (options.clone(), *min, *max, *prompt)
        }
        other => panic!("expected a card question, got {other:?}"),
    }
}

#[track_caller]
fn answer(engine: &mut Bench, objects: Vec<ObjectId>) {
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects })
        .expect("the engine takes an answer to its own question");
}

/// Sacrifices every Dredger `seat` controls, so their dredge works from the
/// graveyard.
fn bury_dredgers(engine: &mut Bench) -> Vec<ObjectId> {
    let dredgers = objects(engine, ZoneLocation::Battlefield, DREDGER);
    for &dredger in &dredgers {
        activate(engine, me(), dredger, 1);
        settle(engine, me());
    }
    for &dredger in &dredgers {
        assert_eq!(
            engine.state().object(dredger).map(|o| o.zone),
            Some(crate::zone::Zone::Graveyard),
            "the Dredger is in its graveyard"
        );
    }
    dredgers
}

/// Activates the Drawer's "draw a card" (index 0) or "draw two" (index 1)
/// and lets it resolve up to its first question.
fn draw_with(engine: &mut Bench, index: u32) {
    let drawer = the(engine, ZoneLocation::Battlefield, DRAWER);
    activate(engine, me(), drawer, index);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass");
}

/// CR 702.52a: "if you would draw a card, you may instead mill N cards and
/// return this card from your graveyard to your hand." Asked as the draw
/// would happen, of the drawing player, with the dredge card on the menu
/// and naming none allowed.
#[test]
fn dredge_mills_and_returns_the_card_instead_of_drawing() {
    let mut engine = game(Seat::with(&[DREDGER, DRAWER]));
    let dredger = bury_dredgers(&mut engine)[0];
    let (library_before, drew_before) = (library(&engine, me()), draws(&engine, me()));
    draw_with(&mut engine, 0);
    assert_eq!(
        asked(&engine),
        (vec![dredger], 0, 1, ChoicePrompt::Dredge),
        "dredge is offered, and may be declined"
    );
    answer(&mut engine, vec![dredger]);
    settle(&mut engine, me());
    assert_eq!(hand(&engine, me()), vec![dredger], "the Dredger came back");
    assert_eq!(
        library(&engine, me()).len(),
        library_before.len() - 2,
        "two cards were milled"
    );
    let top_two = &library_before[library_before.len() - 2..];
    for card in top_two {
        assert!(
            graveyard(&engine, me()).contains(card),
            "the top two cards went to the graveyard"
        );
    }
    assert_eq!(draws(&engine, me()), drew_before, "no card was drawn");
}

/// Declining is drawing: the dredge card stays where it is.
#[test]
fn a_declined_dredge_leaves_the_draw_to_be_made() {
    let mut engine = game(Seat::with(&[DREDGER, DRAWER]));
    let dredger = bury_dredgers(&mut engine)[0];
    let top = *library(&engine, me()).last().expect("a library");
    draw_with(&mut engine, 0);
    assert_eq!(asked(&engine).3, ChoicePrompt::Dredge);
    answer(&mut engine, vec![]);
    settle(&mut engine, me());
    assert_eq!(hand(&engine, me()), vec![top], "the top card was drawn");
    assert!(graveyard(&engine, me()).contains(&dredger));
}

/// CR 702.52b: "A player with fewer cards in their library than the number
/// required by a dredge ability can't mill any of them this way." With one
/// card left, a dredge 2 is not offered; with two it is.
#[test]
fn dredge_needs_as_many_cards_in_the_library_as_it_mills() {
    let mut engine = game(Seat::with(&[DREDGER, DRAWER, MILLER]));
    bury_dredgers(&mut engine);
    let left = library(&engine, me()).len();
    assert!(left > 57 + 1, "room to mill down to one or two: {left}");
    // Down to two cards first, so the control comes before the refusal.
    let miller = the(&engine, ZoneLocation::Battlefield, MILLER);
    let extra = left - 57 - 2;
    for _ in 0..extra {
        // A Drawer draw with the dredge declined takes one card at a time.
        draw_with(&mut engine, 0);
        answer(&mut engine, vec![]);
        settle(&mut engine, me());
    }
    activate(&mut engine, me(), miller, 0);
    settle(&mut engine, me());
    assert_eq!(library(&engine, me()).len(), 2);
    draw_with(&mut engine, 0);
    assert_eq!(asked(&engine).3, ChoicePrompt::Dredge, "two cards: offered");
    answer(&mut engine, vec![]);
    settle(&mut engine, me());
    assert_eq!(library(&engine, me()).len(), 1);
    let hand_before = hand(&engine, me()).len();
    draw_with(&mut engine, 0);
    settle(&mut engine, me());
    assert_eq!(
        hand(&engine, me()).len(),
        hand_before + 1,
        "one card left: the draw is made without a question"
    );
}

/// CR 121.2: each card of "draw two cards" is a draw of its own, and
/// another dredge card may replace the second (the cards' ruling).
#[test]
fn each_card_of_a_multiple_draw_is_offered_dredge_on_its_own() {
    let mut engine = game(Seat::with(&[DREDGER, DREDGER, DRAWER]));
    let dredgers = bury_dredgers(&mut engine);
    draw_with(&mut engine, 1);
    let (options, ..) = asked(&engine);
    assert_eq!(options.len(), 2, "both dredge cards are offered");
    answer(&mut engine, vec![dredgers[0]]);
    let (options, ..) = asked(&engine);
    assert_eq!(
        options,
        vec![dredgers[1]],
        "the second draw is asked about on its own, with what is left"
    );
    answer(&mut engine, vec![dredgers[1]]);
    settle(&mut engine, me());
    let mut back = hand(&engine, me());
    back.sort_unstable();
    let mut expected = dredgers.clone();
    expected.sort_unstable();
    assert_eq!(back, expected, "both came back, nothing was drawn");
}

/// Chains: "except the first one they draw in each of their draw steps".
/// The opponent's own draw-step draw is left alone; a second draw in the
/// same turn, here an effect's in their main phase, is replaced: they
/// discard, then draw.
#[test]
fn chains_spare_the_first_draw_of_a_draw_step_and_replace_the_next() {
    let mut engine = bench(
        7700,
        cards(),
        [
            Seat::with(&[CHAINS]),
            Seat::with(&[DRAWER]).holding(&[BAUBLE, BAUBLE]),
        ],
    );
    to_main(&mut engine, them());
    assert_eq!(
        hand(&engine, them()).len(),
        3,
        "the draw-step draw was made, nothing discarded"
    );
    assert!(graveyard(&engine, them()).is_empty());
    let top = *library(&engine, them()).last().expect("a library");
    let drawer = the(&engine, ZoneLocation::Battlefield, DRAWER);
    activate(&mut engine, them(), drawer, 0);
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("Chains ask for a discard: {:?}", engine.pending())
    };
    assert_eq!(player, them(), "the drawing player discards");
    assert_eq!((min, max, prompt), (1, 1, ChoicePrompt::Discard));
    assert_eq!(options, hand(&engine, them()), "any card in their hand");
    let bauble = options[0];
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![bauble],
            },
        )
        .expect("the discard");
    settle(&mut engine, them());
    assert_eq!(graveyard(&engine, them()), vec![bauble], "discarded");
    assert!(
        hand(&engine, them()).contains(&top),
        "and then the card was drawn"
    );
}

/// With an empty hand the Chains mill a card and no card is drawn (the
/// card's ruling) — not even from an empty library a loss would follow.
#[test]
fn chains_with_an_empty_hand_mill_instead_of_drawing() {
    let mut engine = game(Seat::with(&[CHAINS, DRAWER]));
    assert!(hand(&engine, me()).is_empty());
    let top = *library(&engine, me()).last().expect("a library");
    let drew = draws(&engine, me());
    draw_with(&mut engine, 0);
    settle(&mut engine, me());
    assert!(hand(&engine, me()).is_empty(), "nothing was drawn");
    assert_eq!(
        graveyard(&engine, me()),
        vec![top],
        "the top card was milled"
    );
    assert_eq!(draws(&engine, me()), drew);
}

/// Two Chains, one card in hand: the first discards it and leaves a draw,
/// which the second replaces with a mill because the hand is empty now
/// (the card's ruling: "Once the player fails to discard and instead
/// mills a card, all further effects … will not do anything").
#[test]
fn two_chains_discard_then_mill() {
    let mut engine = game(Seat::with(&[CHAINS, CHAINS, DRAWER]).holding(&[BAUBLE]));
    let bauble = hand(&engine, me())[0];
    let top = *library(&engine, me()).last().expect("a library");
    draw_with(&mut engine, 0);
    settle(&mut engine, me());
    assert!(hand(&engine, me()).is_empty(), "no card was drawn");
    let mut grave = graveyard(&engine, me());
    grave.sort_unstable();
    let mut expected = vec![bauble, top];
    expected.sort_unstable();
    assert_eq!(grave, expected, "the Bauble discarded, the top card milled");
}

/// The draw a Chains leave is a draw (CR 616.2): a dredge card the Chains
/// just made the player discard may replace it.
#[test]
fn the_draw_chains_leave_may_be_dredged_with_the_card_they_discarded() {
    let mut engine = game(Seat::with(&[CHAINS, DRAWER]).holding(&[DREDGER, BAUBLE]));
    let dredger = the(&engine, ZoneLocation::Hand(me()), DREDGER);
    draw_with(&mut engine, 0);
    let (options, min, max, prompt) = asked(&engine);
    assert_eq!((min, max, prompt), (1, 1, ChoicePrompt::Discard));
    assert!(options.contains(&dredger));
    answer(&mut engine, vec![dredger]);
    assert_eq!(
        asked(&engine),
        (vec![dredger], 0, 1, ChoicePrompt::Dredge),
        "the draw left behind is offered to dredge"
    );
    answer(&mut engine, vec![dredger]);
    settle(&mut engine, me());
    assert!(hand(&engine, me()).contains(&dredger), "dredged back");
    assert_eq!(hand(&engine, me()).len(), 2, "the Bauble and the Dredger");
}

/// CR 616.1: with a dredge card in the graveyard and Chains in play, the
/// drawing player chooses which applies first. Dredging first leaves no
/// draw for the Chains: nothing is discarded.
#[test]
fn dredge_or_chains_first_is_the_drawing_players_choice() {
    let mut engine = game(Seat::with(&[CHAINS, DREDGER, DRAWER]).holding(&[BAUBLE, BAUBLE]));
    let chains = the(&engine, ZoneLocation::Battlefield, CHAINS);
    let dredger = bury_dredgers(&mut engine)[0];
    draw_with(&mut engine, 0);
    let (mut options, min, max, prompt) = asked(&engine);
    options.sort_unstable();
    let mut expected = vec![dredger, chains];
    expected.sort_unstable();
    assert_eq!(
        (options, min, max, prompt),
        (expected, 1, 1, ChoicePrompt::DrawReplacement)
    );
    answer(&mut engine, vec![dredger]);
    settle(&mut engine, me());
    assert!(hand(&engine, me()).contains(&dredger));
    assert_eq!(
        graveyard(&engine, me()).len(),
        2,
        "two milled, no Bauble discarded"
    );
    assert_eq!(hand(&engine, me()).len(), 3);
}

/// Aladdin's Lamp: the next draw looks at the top three, two of them go to
/// the bottom, and the one left on top is drawn. The draw after that is an
/// ordinary one: the effect is spent.
#[test]
fn the_lamp_keeps_one_of_the_top_cards_for_the_next_draw_only() {
    let mut engine = game(Seat::with(&[LAMP, DRAWER]));
    let lamp = the(&engine, ZoneLocation::Battlefield, LAMP);
    activate(&mut engine, me(), lamp, 0);
    settle(&mut engine, me());
    let before = library(&engine, me());
    let top_three: Vec<ObjectId> = before.iter().rev().take(3).copied().collect();
    draw_with(&mut engine, 0);
    let (mut options, min, max, prompt) = asked(&engine);
    options.sort_unstable();
    let mut expected = top_three.clone();
    expected.sort_unstable();
    assert_eq!(
        (options, min, max, prompt),
        (expected, 2, 2, ChoicePrompt::PutOnBottom),
        "all but one of the top three go to the bottom"
    );
    // Keep the third card from the top.
    answer(&mut engine, vec![top_three[0], top_three[1]]);
    settle(&mut engine, me());
    assert_eq!(hand(&engine, me()), vec![top_three[2]], "the kept card");
    let after = library(&engine, me());
    let mut bottom: Vec<ObjectId> = after.iter().take(2).copied().collect();
    bottom.sort_unstable();
    let mut sent = vec![top_three[0], top_three[1]];
    sent.sort_unstable();
    assert_eq!(bottom, sent, "the other two are at the bottom");
    let next = *after.last().expect("a library");
    draw_with(&mut engine, 0);
    settle(&mut engine, me());
    assert!(
        hand(&engine, me()).contains(&next),
        "the next draw is plain"
    );
}

/// CR 616.1: two replacements that both must apply and whose order
/// matters (Lamp and Chains) are ordered by the drawing player. Lamp
/// first: it looks and keeps a card; the draw it leaves meets the Chains,
/// which discard the Bauble before the kept card is drawn.
#[test]
fn the_drawing_player_orders_two_replacements_that_must_apply() {
    let mut engine = game(Seat::with(&[LAMP, CHAINS, DRAWER]).holding(&[BAUBLE]));
    let lamp = the(&engine, ZoneLocation::Battlefield, LAMP);
    let chains = the(&engine, ZoneLocation::Battlefield, CHAINS);
    let bauble = hand(&engine, me())[0];
    activate(&mut engine, me(), lamp, 0);
    settle(&mut engine, me());
    let top_three: Vec<ObjectId> = library(&engine, me())
        .iter()
        .rev()
        .take(3)
        .copied()
        .collect();
    draw_with(&mut engine, 0);
    let (mut options, min, max, prompt) = asked(&engine);
    options.sort_unstable();
    let mut expected = vec![lamp, chains];
    expected.sort_unstable();
    assert_eq!(
        (options, min, max, prompt),
        (expected, 1, 1, ChoicePrompt::DrawReplacement)
    );
    answer(&mut engine, vec![lamp]);
    assert_eq!(asked(&engine).3, ChoicePrompt::PutOnBottom);
    answer(&mut engine, vec![top_three[0], top_three[1]]);
    settle(&mut engine, me());
    assert_eq!(
        graveyard(&engine, me()),
        vec![bauble],
        "the Chains discarded"
    );
    assert_eq!(
        hand(&engine, me()),
        vec![top_three[2]],
        "the kept card drawn"
    );
}

/// "Draw a card and reveal it" whose draw waited for a declined dredge
/// still reveals the card it drew; a dredged draw drew nothing to reveal
/// (CR 614.11b).
#[test]
fn a_card_drawn_after_a_declined_dredge_is_still_revealed() {
    for dredge in [false, true] {
        let mut engine = game(Seat::with(&[DREDGER, SEER]));
        let dredger = bury_dredgers(&mut engine)[0];
        let top = *library(&engine, me()).last().expect("a library");
        let seer = the(&engine, ZoneLocation::Battlefield, SEER);
        activate(&mut engine, me(), seer, 0);
        engine
            .apply(me(), PlayerAction::PassPriority)
            .expect("pass");
        engine
            .apply(them(), PlayerAction::PassPriority)
            .expect("pass");
        assert_eq!(asked(&engine).3, ChoicePrompt::Dredge);
        answer(&mut engine, if dredge { vec![dredger] } else { vec![] });
        settle(&mut engine, me());
        let revealed: Vec<ObjectId> = engine
            .journal()
            .entries()
            .iter()
            .filter_map(|e| match &e.event {
                GameEvent::Revealed { cards, .. } => Some(cards.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        if dredge {
            assert!(revealed.is_empty(), "nothing was drawn: {revealed:?}");
            assert_eq!(hand(&engine, me()), vec![dredger]);
        } else {
            assert_eq!(revealed, vec![top], "the drawn card was revealed");
            assert_eq!(hand(&engine, me()), vec![top], "a land is kept");
        }
    }
}
