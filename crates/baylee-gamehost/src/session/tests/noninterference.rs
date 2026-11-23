//! What a seat or a spectator is sent does not depend on what it may not
//! know. Two games that differ only in one player's hidden cards are played
//! alike, and every frame the others are sent has to be the same, byte for
//! byte: a view, a log line, a printing, a count or an order that tells the
//! two games apart tells the cards.

use super::*;
use crate::view::SPECTATOR;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).expect(name)
}

fn entries(cards: &[(&str, usize)], print: u16) -> Vec<DeckEntry> {
    cards
        .iter()
        .flat_map(|(name, n)| std::iter::repeat_n(named(name), *n))
        .map(|card| DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card,
            print: PrintRef::new(print),
        })
        .collect()
}

/// A duel in which the other player's deck is `theirs`, printed as `print`.
/// Everything that player keeps hidden is in those two.
fn duel_preset(theirs: &[(&str, usize)], print: u128, hand: Option<usize>) -> GamePreset {
    let mut preset = split_preset();
    preset.prints[1] = PrintInfo {
        scryfall_id: uuid::Uuid::from_u128(print),
        lang: if print.is_multiple_of(2) { "DE" } else { "JA" }.into(),
        finish: if print.is_multiple_of(2) {
            Finish::Foil
        } else {
            Finish::Normal
        },
    };
    let deck = entries(theirs, 1);
    let spec = &mut preset.seats[1];
    spec.starting_battlefield = vec![];
    spec.starting_hand = hand.map(|n| deck[..n].to_vec());
    spec.deck = deck;
    if let Some(n) = hand {
        preset.seats[0].starting_hand = Some(preset.seats[0].deck[..n].to_vec());
    }
    preset
}

fn seated(preset: &GamePreset) -> Session {
    let mut session = Session::new(preset).expect("a duel");
    session.describe("game".into(), vec!["Me".into(), "Them".into()]);
    session.set_spectators(1);
    session
}

fn duel(theirs: &[(&str, usize)], print: u128, hand: Option<usize>) -> Session {
    seated(&duel_preset(theirs, print, hand))
}

type Frames = Vec<(PlayerId, Envelope)>;

fn to(frames: &Frames, seat: PlayerId) -> Vec<&Envelope> {
    frames
        .iter()
        .filter(|(to, _)| *to == seat)
        .map(|(_, frame)| frame)
        .collect()
}

/// Two games side by side, and whether the hidden difference ever reached
/// the one who may know it.
struct Pair {
    one: Session,
    two: Session,
    told_apart: bool,
}

impl Pair {
    fn new(one: Session, two: Session) -> Self {
        let mut pair = Self {
            one,
            two,
            told_apart: false,
        };
        let opening = (pair.one.pump(), pair.two.pump());
        pair.same(&opening.0, &opening.1);
        pair
    }

    fn same(&mut self, one: &Frames, two: &Frames) {
        for seat in [ME, SPECTATOR] {
            assert_eq!(
                to(one, seat),
                to(two, seat),
                "{seat:?} can tell the two games apart"
            );
        }
        self.told_apart |= to(one, THEM) != to(two, THEM);
        // What a socket that opens now is sent, too.
        assert_eq!(self.one.game_static(ME), self.two.game_static(ME));
        assert_eq!(self.one.snapshot(ME), self.two.snapshot(ME));
        assert_eq!(
            self.one.spectator_game_static(),
            self.two.spectator_game_static()
        );
        assert_eq!(self.one.spectator_view(), self.two.spectator_view());
    }

    /// The seat asked now answers with what does nothing, in both games.
    fn step(&mut self) {
        let (seat, one) = timeout(&self.one).expect("a question");
        let (other, two) = timeout(&self.two).expect("a question");
        assert_eq!(seat, other, "another seat is asked");
        self.act(seat, one, two);
    }

    fn act(&mut self, seat: PlayerId, one: PlayerAction, two: PlayerAction) {
        let one = self.one.act(seat, one).expect("a legal answer");
        let two = self.two.act(seat, two).expect("a legal answer");
        self.same(&one, &two);
    }

    fn turn(&self) -> u32 {
        self.one.state().turn.number
    }
}

#[test]
fn a_hand_and_a_library_change_nothing_anyone_else_is_sent() {
    let forests = duel(&[("Forest", 60)], 2, Some(1));
    let others = duel(
        &[("Lightning Bolt", 20), ("Opt", 20), ("Island", 20)],
        3,
        Some(1),
    );
    let mut pair = Pair::new(forests, others);
    for _ in 0..400 {
        if pair.turn() > 6 {
            break;
        }
        pair.step();
    }
    assert!(pair.turn() > 6, "six turns, three draws of theirs");
    assert!(pair.told_apart, "their own view told their cards");
}

#[test]
fn a_mulligan_tells_its_count_and_none_of_its_cards() {
    let mut pair = Pair::new(
        duel(&[("Forest", 60)], 2, None),
        duel(&[("Lightning Bolt", 30), ("Island", 30)], 3, None),
    );
    pair.act(THEM, PlayerAction::MulliganTake, PlayerAction::MulliganTake);
    for _ in 0..40 {
        if pair.turn() > 1 {
            break;
        }
        pair.step();
    }
    assert!(pair.turn() > 1, "past the mulligans and my turn");
    assert!(pair.told_apart);
}

/// The other side of the claim: what is played is known, so the frames that
/// stayed the same were able to differ.
#[test]
fn a_card_that_is_played_is_told() {
    let mut one = duel(&[("Forest", 60)], 2, Some(1));
    let mut two = duel(&[("Island", 60)], 3, Some(1));
    let mut played = [Frames::new(), Frames::new()];
    for (session, frames) in [&mut one, &mut two].into_iter().zip(&mut played) {
        let _ = session.pump();
        for _ in 0..100 {
            let seat = session.awaiting_seat().expect("a question");
            let hand = crate::view::own_hand(session.state(), THEM);
            let land = hand.first().map(|card| card.id);
            if seat == THEM
                && session.state().turn.active == THEM
                && let Some(card) = land
                && let Ok(out) = session.act(THEM, PlayerAction::PlayLand { card })
            {
                *frames = out;
                break;
            }
            let (seat, action) = timeout(session).expect("a question");
            session.act(seat, action).expect("a legal answer");
        }
    }
    for seat in [ME, SPECTATOR] {
        assert!(!to(&played[0], seat).is_empty());
        assert_ne!(to(&played[0], seat), to(&played[1], seat), "{seat:?}");
    }
}

/// Lands that can pay for an instant are on the table in both games, and the
/// instant is in only one of the hands: whether its player is asked, and how
/// often, must not say so.
#[test]
fn a_card_that_could_be_cast_now_is_not_told_by_who_is_asked() {
    let lands = entries(&[("Island", 2), ("Mountain", 2)], 0);
    let table = |theirs: &[(&str, usize)], print| {
        let mut preset = duel_preset(theirs, print, Some(2));
        preset.seats[1].starting_battlefield.clone_from(&lands);
        seated(&preset)
    };
    let mut pair = Pair::new(
        table(&[("Forest", 60)], 2),
        table(
            &[("Opt", 2), ("Counterspell", 20), ("Lightning Bolt", 38)],
            3,
        ),
    );
    for _ in 0..400 {
        if pair.turn() > 5 {
            break;
        }
        pair.step();
    }
    assert!(pair.turn() > 5);
    assert!(pair.told_apart);
}

/// A card everyone sees being cast looks at a library nobody else may: the
/// look, what is done with it and the card drawn stay the caster's.
#[test]
fn a_look_at_the_library_tells_nobody_else_what_was_on_top() {
    let table = |theirs: &[(&str, usize)], print| {
        let mut preset = duel_preset(theirs, print, Some(0));
        let spec = &mut preset.seats[1];
        spec.starting_hand = Some(entries(&[("Opt", 1)], 0));
        spec.starting_battlefield = entries(&[("Island", 1)], 0);
        seated(&preset)
    };
    let mut pair = Pair::new(
        table(&[("Forest", 60)], 2),
        table(&[("Lightning Bolt", 30), ("Mountain", 30)], 3),
    );
    let island = (0..100).find_map(|_| {
        if let Pending::Priority { player, legal } = pair.one.pending()
            && *player == THEM
            && pair.one.state().turn.active == THEM
            && let Some(source) = legal.mana_abilities.first()
        {
            return Some(*source);
        }
        pair.step();
        None
    });
    let source = island.expect("their Island can be tapped");
    let tap = PlayerAction::ActivateManaAbility { source };
    pair.act(THEM, tap.clone(), tap);
    let card = seat_view(&pair.one, THEM).hand[0].id;
    let cast = PlayerAction::CastSpell { card };
    pair.act(THEM, cast.clone(), cast);
    let mut looked = false;
    for _ in 0..100 {
        looked |= matches!(
            pair.one.pending(),
            Pending::Arrange { player, .. } if *player == THEM
        );
        if pair.turn() > 3 {
            break;
        }
        pair.step();
    }
    assert!(looked, "Opt looks at the top card");
    assert!(pair.told_apart && pair.turn() > 3);
}
