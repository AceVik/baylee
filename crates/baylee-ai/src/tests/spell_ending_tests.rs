//! Where the house sends its own resolved spell when more than one "instead"
//! wants it (CR 616.1e): what `crate::endings` says, asked as the engine asks.

use super::*;
use baylee_engine::choice::{CastModeDesc, CastModeKind, SpellEnding};

const ME: PlayerId = PlayerId::new(0);

fn land(slot: u32) -> PublicObject {
    let mut object = permanent(obj(slot), ME, 0);
    object.types = TypeSet::LAND;
    object.power = None;
    object.toughness = None;
    object
}

/// `name` of the house's on the stack, on its way out.
fn resolved(name: &str, face: u8) -> PublicObject {
    let mut o = permanent(obj(1), ME, 0);
    o.card = Some(hand_card(1, name).card);
    o.rules = o.card.map(|card| baylee_view::RulesFace {
        face,
        ..baylee_view::RulesFace::from(card)
    });
    o.types = TypeSet::INSTANT;
    o.power = None;
    o.toughness = None;
    o.stack_item = Some(baylee_view::StackItem::Spell);
    o
}

fn question(endings: &[SpellEnding]) -> Pending {
    Pending::ChooseCastMode {
        player: ME,
        object: obj(1),
        options: endings
            .iter()
            .zip(0..)
            .map(|(ending, index)| CastModeDesc {
                index,
                kind: CastModeKind::SpellEnding(*ending),
                cost: baylee_core::mana::ManaCost::ZERO,
            })
            .collect(),
    }
}

/// What the house answers with `lands` lands and `spell` resolved.
fn chosen(lands: u32, spell: PublicObject, endings: &[SpellEnding]) -> SpellEnding {
    let mut v = view(0, &[20, 20], (10..10 + lands).map(land).collect());
    v.stack.push(spell);
    let PlayerAction::ChooseMode(at) = agent().act(&v, &question(endings)) else {
        panic!("an ending is answered with a position");
    };
    endings[at]
}

const HAND_OR_REBOUND: [SpellEnding; 2] = [SpellEnding::Hand, SpellEnding::Rebound];

/// Capsize is {1}{U}{U} and {3} again: six lands keep the card turning.
#[test]
fn a_seat_that_can_pay_for_it_again_keeps_the_card() {
    assert_eq!(
        chosen(6, resolved("Capsize", 0), &HAND_OR_REBOUND),
        SpellEnding::Hand
    );
}

/// One land short, the card in hand would sit there: the free cast is worth
/// more than a card it cannot pay for.
#[test]
fn a_seat_that_cannot_takes_the_free_cast() {
    assert_eq!(
        chosen(5, resolved("Capsize", 0), &HAND_OR_REBOUND),
        SpellEnding::Rebound
    );
}

/// The next upkeep has nothing on the stack for Forbid to answer, so its free
/// cast is worth nothing however poor the seat is.
#[test]
fn a_counterspell_goes_home_however_few_the_lands() {
    assert_eq!(
        chosen(0, resolved("Forbid", 0), &HAND_OR_REBOUND),
        SpellEnding::Hand
    );
}

/// An exile that gives nothing back is taken only when nothing else is left.
#[test]
fn a_plain_exile_is_never_taken_over_anything_else() {
    let spell = || resolved("Capsize", 0);
    assert_eq!(
        chosen(0, spell(), &[SpellEnding::Hand, SpellEnding::Exile]),
        SpellEnding::Hand
    );
    assert_eq!(
        chosen(0, spell(), &[SpellEnding::Rebound, SpellEnding::Exile]),
        SpellEnding::Rebound
    );
    assert_eq!(
        chosen(
            0,
            resolved("Murderous Rider", 1),
            &[SpellEnding::Adventure, SpellEnding::Exile]
        ),
        SpellEnding::Adventure
    );
}

/// Rebound casts the creature free next upkeep; an adventure only lets it be
/// paid for.
#[test]
fn the_free_creature_beats_paying_for_it_off_an_adventure() {
    assert_eq!(
        chosen(
            8,
            resolved("Murderous Rider", 1),
            &[SpellEnding::Rebound, SpellEnding::Adventure]
        ),
        SpellEnding::Rebound
    );
}

/// A cast question is not an ending: the position the old path picks stands.
#[test]
fn a_way_to_cast_is_still_answered_as_one() {
    let mut v = view(0, &[20, 20], vec![]);
    v.stack.push(resolved("Capsize", 0));
    let cast = Pending::ChooseCastMode {
        player: ME,
        object: obj(1),
        options: vec![
            CastModeDesc {
                index: 0,
                kind: CastModeKind::Kicked,
                cost: baylee_core::mana::ManaCost::ZERO,
            },
            CastModeDesc {
                index: 1,
                kind: CastModeKind::Normal,
                cost: baylee_core::mana::ManaCost::ZERO,
            },
        ],
    };
    assert_eq!(agent().act(&v, &cast), PlayerAction::ChooseMode(1));
}
