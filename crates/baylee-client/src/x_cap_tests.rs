//! The X question of a spell whose cost the board pays beside mana: convoke's
//! creatures (CR 702.51a) count toward the cap the client puts on X
//! (`Duel::cap_x`), as the engine counts them.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::types::TypeSet;
use baylee_engine::choice::LegalActions;

const ME: PlayerId = PlayerId::new(0);

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

fn forest(slot: u32) -> baylee_view::PublicObject {
    let mut o = token(slot, 0, "Forest", 0, 0);
    o.types = TypeSet::LAND;
    o.power = None;
    o.toughness = None;
    o.subtypes.insert(land::FOREST);
    o
}

fn elf(slot: u32) -> baylee_view::PublicObject {
    let mut o = token(slot, 0, "Elf", 1, 1);
    o.types = TypeSet::CREATURE;
    o.colors = ColorSet::of(Color::Green);
    o
}

/// Carbon b4, attempt 8, q158, as the client would have asked it: Chord of
/// Calling (`{X}{G}{G}{G}`, convoke) beside five Forests and three green
/// creatures pays X = 5; the cap that counted the Forests alone stopped at
/// 2.
#[test]
fn a_convoke_spells_x_is_capped_by_the_lands_and_the_creatures() {
    let chord = baylee_cards::decks::by_name("Chord of Calling").expect("Chord of Calling");
    let view = || {
        let mut view = ViewBuilder::new(2)
            .with_hand(vec![("Chord of Calling", 3, 20)])
            .with_battlefield(
                0,
                [
                    forest(31),
                    forest(32),
                    forest(33),
                    forest(34),
                    forest(35),
                    elf(41),
                    elf(42),
                    elf(43),
                ],
            )
            .build();
        view.hand[0].card.index = chord;
        view
    };
    let priority = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable: vec![obj(20)],
            mana_abilities: (31..=35).map(obj).collect(),
            ..LegalActions::default()
        }),
    };
    let mut duel = Duel::default();
    duel.receive_view(view());
    duel.receive_choice(priority.clone());
    duel.receive_choice(priority);
    let mut casting = view();
    casting.casting = Some(obj(20));
    duel.receive_view(casting);
    duel.receive_choice(Pending::ChooseNumber {
        player: ME,
        min: 0,
        max: u32::MAX,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    assert_eq!(
        duel.interaction.as_mut().unwrap().set_number(100),
        5,
        "five Forests and three convoking Elves pay X = 5"
    );
}
