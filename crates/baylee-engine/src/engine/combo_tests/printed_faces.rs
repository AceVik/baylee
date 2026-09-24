//! Which card an object's abilities are printed on, carried beside the list
//! through every place the engine sets one aside (`PrintedFace`).
//!
//! The engine never reads it; a client does, because the sentence it draws
//! for an ability row or a stack entry is the one printed on *that* card. A
//! copy's is the copied card's (CR 707.2), an ability on the stack keeps its
//! source's (CR 113.7a), and a look-back trigger keeps what its source was
//! the moment before it left (CR 603.10a). Each of those reaches the list
//! through different code, so each is played here, and the bystander in each
//! is the object that must still name **itself**: the original beside its
//! copy, and the copy once it has stopped being one.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use crate::object::{GameObject, ObjectKind, PrintedFace};

/// The front face of `card`.
fn front(card: baylee_core::ids::CardIndex) -> Option<PrintedFace> {
    PrintedFace::new(card, 0)
}

/// An ability on the stack whose list is printed on `card`, if one is there.
fn stacked_from(
    engine: &Engine<RegistryLookup>,
    card: baylee_core::ids::CardIndex,
) -> Option<baylee_core::ids::ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.kind == ObjectKind::AbilityOnStack && o.printed_face() == front(card)
            })
        })
}

/// A Phyrexian Metamorph that entered as their Solemn Simulacrum: the copy,
/// its enters trigger and — once it has died and stopped being a copy — its
/// dies trigger all name Solemn, while the card in the graveyard names the
/// Metamorph again.
///
/// The dies trigger is the one that crosses every hand-off at once: the list
/// is kept on the state as the copy leaves (`ltb_abilities`), rides the
/// queued trigger, is laid in the activation slot and is taken by the
/// ability object. A face dropped at any of the four would read here as the
/// Metamorph's, because by then the Metamorph is all the source still is.
#[test]
fn a_copy_and_both_of_its_triggers_name_the_card_they_copied() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(77, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[phyrexian_metamorph()])
        .battlefield(
            1,
            &[solemn_simulacrum(), plains(), plains(), swamp(), swamp()],
        )
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, phyrexian_metamorph());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let original = on_battlefield(&engine, p1, solemn_simulacrum()).expect("their Golem");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![original],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        stacked_from(e, solemn_simulacrum()).is_some()
            || matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let enters = stacked_from(&engine, solemn_simulacrum()).expect("the copied enters trigger");
    let loc = engine
        .state()
        .object(enters)
        .and_then(|o| o.ability)
        .expect("an ability on the stack says which it is");
    assert_eq!(
        loc.card,
        Some(phyrexian_metamorph()),
        "the handle a standing answer is filed under is still the card on the table"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, p0, phyrexian_metamorph()).expect("the copy arrived");
    assert_eq!(
        engine
            .state()
            .object(copy)
            .and_then(GameObject::printed_face),
        front(solemn_simulacrum()),
        "a copy's abilities are the copied card's"
    );
    assert_eq!(
        engine
            .state()
            .object(original)
            .and_then(GameObject::printed_face),
        front(solemn_simulacrum()),
        "and the Golem it copied names itself, as it always did"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![copy],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        stacked_from(e, solemn_simulacrum()).is_some()
    });
    let card = engine
        .state()
        .object(copy)
        .expect("the Metamorph is in a zone");
    assert_eq!(card.zone, crate::zone::Zone::Graveyard, "destroyed");
    assert_eq!(
        card.printed_face(),
        front(phyrexian_metamorph()),
        "the card in the graveyard is the printed Metamorph (CR 400.7)"
    );
}

/// An activation whose cost sacrifices the copy: the ability names the card
/// the copy was of, although by the time it is on the stack its source is a
/// Glasspool Mimic in the graveyard.
///
/// The activation path sets the list aside before any cost is paid, which is
/// the one hand-off the trigger test above does not cross.
#[test]
fn an_ability_a_copy_paid_for_with_its_life_names_the_card_it_copied() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(75, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                plains(),
                plains(),
                werefox_bodyguard(),
            ],
        )
        .hand(0, &[glasspool_mimic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fox = on_battlefield(&engine, p0, werefox_bodyguard()).expect("the bodyguard is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let mimic = in_hand(&engine, p0, glasspool_mimic()).expect("mimic in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mimic })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![fox] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    let index = offered_ability(&engine, mimic).expect("the copy carries what it copied");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mimic,
                ability_index: index,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        stacked_from(e, werefox_bodyguard()).is_some()
    });
    let source = engine.state().object(mimic).expect("the copy is in a zone");
    assert_eq!(source.zone, crate::zone::Zone::Graveyard, "sacrificed");
    assert_eq!(
        source.printed_face(),
        front(glasspool_mimic()),
        "the source has stopped being a copy; the ability has not"
    );
    assert_eq!(
        engine
            .state()
            .object(fox)
            .and_then(GameObject::printed_face),
        front(werefox_bodyguard()),
        "and the Fox it copied names itself"
    );
}

/// A token copy has no card at all, so what it is printed on is the one
/// thing about it that names the creature it copied. Its list arrives a step
/// after the token does (`settle_copied_rules_text`), and the face has to
/// arrive with it.
#[test]
fn a_token_copy_names_the_card_it_copies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(96, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[rite_of_replication()])
        .battlefield(1, &[baleful_strix(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let victim = aim_at_theirs(&mut engine, rite_of_replication(), baleful_strix(), false);
    let copy = the_copy_on(&engine, p0);
    let token = engine.state().object(copy).expect("the copy");
    assert!(token.card.is_none(), "a token (CR 707.10)");
    assert_eq!(token.printed_face(), front(baleful_strix()));
    assert_eq!(
        engine
            .state()
            .object(victim)
            .and_then(GameObject::printed_face),
        front(baleful_strix()),
        "their Strix is printed on itself"
    );
    let mine = on_battlefield(&engine, p0, island()).expect("my Island");
    assert_eq!(
        engine
            .state()
            .object(mine)
            .and_then(GameObject::printed_face),
        front(island()),
        "and a card that copied nothing names its own face"
    );
}

/// A copy that ends with the turn gives the face back with the list: the
/// Cursed Mirror is a Llanowar Elf until the cleanup step and a Cursed Mirror
/// after it.
#[test]
fn a_temporary_copy_names_the_elf_only_while_it_is_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, mirror) = a_mirror_that_became_their_elf(101);
    assert_eq!(
        engine
            .state()
            .object(mirror)
            .and_then(GameObject::printed_face),
        front(llanowar_elves())
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine
            .state()
            .object(mirror)
            .and_then(GameObject::printed_face),
        front(cursed_mirror()),
        "the cleanup step took the Elf's text back, and its face with it"
    );
}

/// The packing is lossless over every card in the pool and every face it
/// prints, and refuses what it cannot hold rather than storing another card.
#[test]
fn a_printed_face_packs_every_card_in_the_pool_and_refuses_what_does_not_fit() {
    use baylee_core::ids::CardIndex;
    let mut faces = 0usize;
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            let face = u8::try_from(face).expect("a card has a handful of faces");
            let packed = PrintedFace::new(def.index, face).expect("every printed face fits");
            assert_eq!((packed.card(), packed.face()), (def.index, face));
            faces += 1;
        }
    }
    assert!(faces >= 2700, "only {faces} faces were packed");
    assert_eq!(PrintedFace::new(CardIndex::new(0), 8), None, "a ninth face");
    assert_eq!(
        PrintedFace::new(CardIndex::new(1 << 29), 0),
        None,
        "an index whose top bits the shift would lose"
    );
    assert_eq!(
        PrintedFace::new(CardIndex::new(u32::MAX >> 3), 7),
        None,
        "the one pair whose packed value is u32::MAX, where the +1 overflows"
    );
    let last = PrintedFace::new(CardIndex::new((u32::MAX >> 3) - 1), 7).expect("fits");
    assert_eq!(
        (last.card(), last.face()),
        (CardIndex::new((u32::MAX >> 3) - 1), 7)
    );
}
