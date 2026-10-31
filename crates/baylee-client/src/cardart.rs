//! Which card a projected name belongs to, and which picture that card wears.
//!
//! The companion to [`crate::tokenart`] and it exists for the same reason: the
//! answer is in the compiled card registry, which `baylee-client-core`
//! deliberately does not link, so the policy stays down there and the lookup
//! is handed in from up here.
//!
//! What needs it is the copy. A permanent that has become a copy of another
//! card keeps its own cardboard — [`baylee_view::PublicObject::card`] is the
//! Clone, in every zone the Clone ever visits — while `name` is the
//! projection. So the view says "Clone" and "Llanowar Elves" in one breath
//! and a client drawing the first under the second was drawing a card that
//! does not exist. A token a copy effect made is the same question with the
//! harder half missing: it has no card and no registry token either, so its
//! name was all there ever was to go on and it drew as a coloured rectangle.

use baylee_cards_dsl::{AbilityDef, ActivationZone};
use baylee_client_core::board::{Registry, Wears};
use baylee_core::ids::CardIndex;
use baylee_view::RulesFace;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The printed card an index wears the picture of.
///
/// `None` for an index no card claims and for a card codegen recorded no
/// printing for, which are both the same answer to the renderer: draw the
/// face rather than fetch a certain 404.
#[must_use]
pub fn of(index: CardIndex) -> Option<&'static str> {
    baylee_cards::by_index(index)
        .map(|c| c.scryfall_id)
        .filter(|s| !s.is_empty())
}

/// The card in the registry that is printed with `name`, and which of its
/// faces carries it.
///
/// A face rather than a card, because a copy of a transformed permanent takes
/// the name of the face that is *up* (CR 707.2 through CR 707.8), and a
/// lookup answering only an index would draw the front of a card the table is
/// showing the back of.
///
/// Front faces are indexed first and a back face only fills a name no front
/// face claimed, so a card whose back happens to share another card's printed
/// name cannot take that name away from it. No name in today's pool is
/// carried by two cards at all, and
/// `every_name_the_registry_prints_leads_back_to_a_picture` is what says so
/// rather than a comment claiming it; the ordering is the answer prepared in
/// advance for the day one appears, and the front is the one a player means.
#[must_use]
pub fn wearing(name: &str) -> Option<(CardIndex, u8)> {
    static BY_NAME: OnceLock<HashMap<&'static str, (CardIndex, u8)>> = OnceLock::new();
    BY_NAME
        .get_or_init(|| {
            let mut map: HashMap<&'static str, (CardIndex, u8)> = HashMap::new();
            for card in baylee_cards::all() {
                if let Some(front) = card.faces.first() {
                    map.entry(front.name).or_insert((card.index, 0));
                }
            }
            for card in baylee_cards::all() {
                for (i, face) in card.faces.iter().enumerate().skip(1) {
                    let face_index = u8::try_from(i).unwrap_or(u8::MAX);
                    map.entry(face.name).or_insert((card.index, face_index));
                }
            }
            map
        })
        .get(name)
        .copied()
}

/// What the registry prints under a name: a card, a token, or nothing.
///
/// A name a token is printed with answers the token, even where a card
/// shares it (Antiquities' Shapeshifter and the Shapeshifter tokens): a name
/// alone cannot tell them apart, and the board asks [`card_by_body`] for the
/// card, which can.
#[must_use]
pub fn named(name: &str) -> Option<Wears> {
    crate::tokenart::wearing(name)
        .map(Wears::Token)
        .or_else(|| wearing(name).map(|(index, face)| Wears::Card(index, face)))
}

/// The card `object` wears, by its projected name, when its body says it is
/// that card.
///
/// Most names are a card's alone and the name is enough, pumped or painted.
/// Where a token shares the name, the card is taken only when the object's
/// card types are the card face's printed ones: a blue 2/2 Shapeshifter
/// creature is the token's body (or nothing), an artifact creature named
/// Shapeshifter is the Antiquities card.
#[must_use]
pub fn card_by_body(object: &baylee_view::PublicObject) -> Option<Wears> {
    let (index, face) = wearing(&object.name)?;
    if crate::tokenart::wearing(&object.name).is_some() {
        let printed = baylee_cards::by_index(index)?
            .faces
            .get(usize::from(face))?;
        if printed.types != object.types {
            return None;
        }
    }
    Some(Wears::Card(index, face))
}

/// What the box of the print `object` is drawn with says: the printed power
/// and toughness of the face [`board::art_of`] draws, read off the registry.
///
/// Not the view's base: that is the copiable body (CR 707.2), which a copy
/// changed by its own rules leaves behind. An eternalized Fanatic of Rhonas
/// is a 4/4 with a 4/4 base, drawn on the Fanatic's print, whose box says
/// 1/4; set against the base, the plate believed the print said 4/4 and was
/// not drawn, and the player read 1/4 (report 30.10.2026). `None` where the
/// face prints no body or no print is drawn.
#[must_use]
pub fn printed_body(object: &baylee_view::PublicObject) -> Option<(i16, i16)> {
    let wears = baylee_client_core::board::worn(object, registry())
        .or_else(|| object.card.map(|c| Wears::Card(c.index, c.face)))
        .or_else(|| object.token.map(Wears::Token))?;
    match wears {
        Wears::Card(index, face) => {
            let face = baylee_cards::by_index(index)?
                .faces
                .get(usize::from(face))?;
            face.power.zip(face.toughness)
        }
        Wears::Token(id) => {
            let token = baylee_cards::tokens::by_token_id(id)?;
            token.power.zip(token.toughness)
        }
    }
}

/// Whether a land is one its player uses for more than mana (#263): the
/// land row's right-hand section.
///
/// The owner's example was "Utility Land rechts", so the test is what a
/// player means by the word, read off the ability list of the face whose
/// rules the land has (a copy follows what it copies), never off its text:
/// an ability activated on the battlefield that is not a mana ability (a
/// fetchland, a creature land; a Triome's cycling is used from the hand), a
/// static or replacement ability (Reliquary Tower, Urborg), or no mana
/// ability at all. What happens *to* its player does not count: entering
/// tapped and the shockland's life are `enter_modifiers`, a bridge's
/// indestructible is a keyword, and City of Brass's damage and Path of
/// Ancestry's scry are a trigger and a rider on mana. An unread ability
/// claims nothing: such a land stands with the mana lands unless what was
/// read already says otherwise.
#[must_use]
pub fn utility_land(face: RulesFace) -> bool {
    let Some(def) = baylee_cards::by_index(face.card) else {
        return false;
    };
    let mut makes_mana = false;
    let mut unread = false;
    for ability in def.abilities_for_face(usize::from(face.face)) {
        match ability {
            ability if ability.is_mana_ability() => makes_mana = true,
            AbilityDef::Activated {
                zone: ActivationZone::Battlefield,
                ..
            }
            | AbilityDef::ActivatedConditional {
                zone: ActivationZone::Battlefield,
                ..
            }
            | AbilityDef::Static(_)
            | AbilityDef::Replacement(_) => return true,
            AbilityDef::Unimplemented => unread = true,
            _ => {}
        }
    }
    !makes_mana && !unread
}

/// The compiled registry, as a board asks for it.
///
/// One function so that no caller can bring half of it. The two lookups are
/// answers to one question asked of two tables, and a board handed a card
/// lookup with no token lookup would draw a copy of a token as its own card
/// and say nothing was wrong.
#[must_use]
pub fn registry() -> Registry<'static> {
    static NAMED: fn(&str) -> Option<Wears> = named;
    static TOKEN_NAME: fn(u16) -> Option<&'static str> = crate::tokenart::name;
    static UTILITY_LAND: fn(RulesFace) -> bool = utility_land;
    // Every picture a board names asks which side of its card it is, and
    // the board is built only through here.
    baylee_client_core::images::know_second_pictures(baylee_cards::sides::has_back_image);
    Registry {
        named: &NAMED,
        token_name: &TOKEN_NAME,
        token_face: &crate::tokenart::matching_body,
        card_face: Some(&card_by_body),
        utility_land: &UTILITY_LAND,
    }
}

#[cfg(test)]
mod tests {
    use super::{named, of, printed_body, registry, utility_land, wearing};
    use baylee_client_core::board::Wears;
    use baylee_view::RulesFace;

    /// A utility land is one its player uses (#263): the lands a player means
    /// by the word stand right, and the ones that only make mana, whatever
    /// else happens to their player, stand in the centre. Zagoth Triome is
    /// here for its cycling, activated from the hand and not on the table.
    #[test]
    fn a_utility_land_is_one_its_player_uses() {
        let face = |name: &str| match named(name) {
            Some(Wears::Card(card, face)) => RulesFace { card, face },
            other => panic!("{name}: {other:?}"),
        };
        for name in [
            "Reliquary Tower",
            "Arid Mesa",
            "Celestial Colonnade",
            "Urborg, Tomb of Yawgmoth",
        ] {
            assert!(utility_land(face(name)), "{name} is a utility land");
        }
        for name in [
            "Forest",
            "Rustvale Bridge",
            "Hallowed Fountain",
            "City of Brass",
            "Path of Ancestry",
            "Tundra",
            "Zagoth Triome",
        ] {
            assert!(!utility_land(face(name)), "{name} only makes mana");
        }
    }

    /// The two halves have to meet: a name is looked up, and the index that
    /// comes back has to be one `of` can turn into a picture. This is the
    /// only place either is read, so a registry that renumbered under them
    /// would be caught nowhere else.
    ///
    /// It asserts that every front face answers with its own card, and any
    /// secondary face that shares a name with another card (such as an SOS
    /// Prepared spell face like `Emeritus of Ideation // Ancestral Recall`)
    /// is resolved by [`wearing`]'s front-first pass to the standalone card's
    /// front face.
    #[test]
    fn every_name_the_registry_prints_leads_back_to_a_picture() {
        let mut checked = 0usize;
        for card in baylee_cards::all() {
            for (face_i, face) in card.faces.iter().enumerate() {
                let Some((index, found_face)) = wearing(face.name) else {
                    panic!("{} is printed and cannot be found by its name", face.name);
                };
                if face_i == 0 {
                    assert_eq!(
                        index, card.index,
                        "{} does not answer with its own card",
                        face.name
                    );
                } else if index != card.index {
                    assert_eq!(found_face, 0);
                    let winner = baylee_cards::by_index(index).expect("winner exists");
                    assert_eq!(winner.faces[0].name, face.name);
                }
                assert!(
                    of(index).is_some(),
                    "{} has a name but no printing to draw",
                    face.name
                );
                checked += 1;
            }
        }
        assert!(checked > 1000, "only {checked} faces were reachable");
    }

    /// The counter-test. A projected name is a `String` off the wire and the
    /// engine may name a token anything at all; an answer for one of those
    /// would be a picture of the wrong card, which is worse than none.
    #[test]
    fn a_name_no_card_is_printed_with_answers_nothing() {
        assert_eq!(wearing("Soldier"), None);
        assert_eq!(wearing(""), None);
        assert_eq!(wearing("Llanowar  Elves"), None, "and it is not fuzzy");
        assert_eq!(of(baylee_core::ids::CardIndex::new(u32::MAX)), None);
    }

    /// No token in the registry is named after a card in it.
    ///
    /// A claim about the *pool*, not about the lookup, and the reason
    /// [`named`] asks the card table first has no consequence today. Were a
    /// card ever printed with a token's name, every one of those chits would
    /// answer with that card's picture and a board full of Soldiers would be
    /// drawn as a board full of whatever the card is — which is why this is a
    /// test and not a sentence in a doc comment.
    #[test]
    fn no_token_is_named_after_a_card() {
        for token in baylee_cards::tokens::ALL {
            let answer = named(token.name);
            assert!(
                matches!(answer, Some(Wears::Token(_))),
                "{} answers {answer:?} rather than a token",
                token.name
            );
        }
    }

    /// The two tables meet in one answer, and a name reaches the right one.
    #[test]
    fn a_name_finds_unambiguous_cards_and_prefers_tokens_on_collisions() {
        let (card, face) = wearing("Counterspell").expect("the pool has it");
        assert_eq!(named("Counterspell"), Some(Wears::Card(card, face)));
        let token = crate::tokenart::wearing("Llanowar Elves")
            .expect("the expanded pool has a named Llanowar Elves token");
        assert!(wearing("Llanowar Elves").is_some());
        assert_eq!(named("Llanowar Elves"), Some(Wears::Token(token)));
        assert_eq!(
            named("Soldier"),
            crate::tokenart::wearing("Soldier").map(Wears::Token),
            "no card is printed Soldier, so the chit answers"
        );
        assert_eq!(named("Not A Card Or A Token"), None);
        assert!((registry().named)("Llanowar Elves").is_some());
    }

    /// The case the whole module exists for, spelled out with a card that is
    /// actually in the pool: what a Clone becomes when it copies one.
    #[test]
    fn the_card_a_copy_wears_is_found_by_the_name_it_projects() {
        let (index, face) = wearing("Llanowar Elves").expect("the pool has Llanowar Elves");
        assert_eq!(face, 0);
        let card = baylee_cards::by_index(index).expect("the index is the registry's own");
        assert_eq!(card.faces[0].name, "Llanowar Elves");
        assert_eq!(of(index), Some(card.scryfall_id));
    }

    #[test]
    fn a_copy_of_the_two_two_shapeshifter_wears_its_body_even_when_pumped() {
        use baylee_client_core::{board, images::ArtSize, test_support::printed};
        let mut copy = printed(1, 0, "Shapeshifter", 3);
        copy.base_power = Some(2);
        copy.base_toughness = Some(2);
        copy.colors = baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Blue]);
        copy.types = baylee_core::types::TypeSet::CREATURE;
        copy.power = Some(5);
        copy.toughness = Some(5);
        let id =
            baylee_cards::tokens::token_id(&baylee_cards::tokens::SHAPESHIFTER_2_2_BLUE_CHANGELING);
        assert_eq!(board::worn(&copy, registry()), Some(Wears::Token(id)));
        assert_eq!(
            board::art_of(&copy, ArtSize::Small, registry()),
            Some(Wears::Token(id).art(ArtSize::Small))
        );
        copy.colors = baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Red]);
        assert_eq!(
            board::worn(&copy, registry()),
            None,
            "wrong colour must not pick a token"
        );
        copy.colors = baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Blue]);
        copy.types = baylee_core::types::TypeSet::ARTIFACT;
        assert_eq!(
            board::worn(&copy, registry()),
            None,
            "wrong type must not pick a token"
        );
        copy.types = baylee_core::types::TypeSet::CREATURE;
        copy.base_power = Some(1);
        copy.base_toughness = Some(1);
        assert_ne!(board::worn(&copy, registry()), Some(Wears::Token(id)));
    }

    /// Antiquities' Shapeshifter shares its name with the Shapeshifter
    /// tokens. A copy of the card (its printed types) wears the card; a body
    /// that is no card's and no token's wears nothing, rather than the card
    /// the name happens to find first.
    #[test]
    fn a_copy_of_the_shapeshifter_card_wears_the_card_and_not_the_token() {
        use baylee_client_core::{board, test_support::printed};
        let (index, face) = wearing("Shapeshifter").expect("Antiquities' Shapeshifter");
        let card = baylee_cards::by_index(index).expect("the registry's own index");
        let mut copy = printed(1, 0, "Shapeshifter", 3);
        copy.types = card.faces[usize::from(face)].types;
        copy.colors = baylee_core::color::ColorSet::EMPTY;
        assert_eq!(
            board::worn(&copy, registry()),
            Some(Wears::Card(index, face)),
            "the card's body wears the card"
        );
        copy.types = baylee_core::types::TypeSet::CREATURE;
        assert_eq!(board::worn(&copy, registry()), None, "neither body");
    }

    /// The key the stack draws for `card` cast with rules face `face`, built
    /// the way the table builds it: a spell object on the stack, a board
    /// model made with the client's own registry, the entry's `art` read back.
    fn stack_picture(name: &str, face: u8) -> baylee_client_core::images::ImageKey {
        use baylee_client_core::board::{BoardModel, Openings};
        use baylee_client_core::test_support::{ViewBuilder, token};
        use baylee_core::ids::PrintRef;
        use baylee_view::CardIdentity;

        let index = baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name}: no card"));
        let identity = CardIdentity {
            index,
            print: PrintRef::new(0),
            face,
        };
        let mut spell = token(40, 0, name, 0, 0);
        spell.card = Some(identity);
        spell.rules = Some(RulesFace::from(identity));
        let view = ViewBuilder::new(2).with_stack(vec![spell]).build();
        let board = BoardModel::from_view(&view, Openings::none(), &[], registry());
        board.stack[0].art.expect("a spell shows its own card")
    }

    /// The URL the key resolves to on a fixed CDN base and a well-formed id.
    fn url_of(key: baylee_client_core::images::ImageKey) -> String {
        baylee_client_core::images::art_url_at(
            "https://cards.example",
            "f333ea01-124f-4125-87ab-609be40e774c",
            key.face,
            key.size,
        )
        .expect("a well-formed id resolves")
    }

    /// An Adventure cast as its Adventure is rules face 1 and still one piece
    /// of cardboard with one picture, so the stack draws the front. It failed
    /// before the fix: `Face::from_index` sent every nonzero face to `/back/`,
    /// which answers 404, and the spell showed no picture.
    #[test]
    fn an_adventure_cast_as_its_adventure_is_drawn_from_the_front() {
        use baylee_client_core::images::Face;
        for name in ["Murderous Rider", "Brazen Borrower"] {
            let key = stack_picture(name, 1);
            assert_eq!(key.face, Face::Front, "{name} has no back picture");
            let url = url_of(key);
            assert!(url.contains("/front/"), "{name}: {url}");
            assert!(!url.contains("/back/"), "{name}: {url}");
        }
    }

    /// A split card cast as its right half is rules face 1 of one picture
    /// too. It failed before the fix, for the Adventure's reason.
    #[test]
    fn a_split_card_cast_as_its_right_half_is_drawn_from_the_front() {
        use baylee_client_core::images::Face;
        let key = stack_picture("Fire // Ice", 1);
        assert_eq!(key.face, Face::Front, "Ice shares Fire's one picture");
        let url = url_of(key);
        assert!(url.contains("/front/") && !url.contains("/back/"), "{url}");
    }

    /// The counter-test, so the table is read both ways: a modal
    /// double-faced card really has a second picture, and its back face is
    /// the `back` shelf while its front stays on the front.
    #[test]
    fn a_modal_double_faced_card_cast_as_its_back_is_drawn_from_the_back() {
        use baylee_client_core::images::Face;
        let front = stack_picture("Agadeem's Awakening", 0);
        let back = stack_picture("Agadeem's Awakening", 1);
        assert_eq!(front.face, Face::Front);
        assert_eq!(back.face, Face::Back);
        assert!(url_of(back).contains("/back/"));
        assert!(url_of(front).contains("/front/"));
    }

    /// A copy whose own rules changed its body is drawn on the copied
    /// card's print, and the plate is set against that print's box, not
    /// against the copiable base (report 30.10.2026): an eternalized Fanatic
    /// of Rhonas is a 4/4 black Zombie with a 4/4 base on a print saying
    /// 1/4, so the plate is written; the card itself on its print is not.
    #[test]
    fn a_copy_whose_body_differs_from_its_print_wears_the_plate() {
        use baylee_client_core::cardplate::Corner;
        let fanatic = baylee_core::generated::index::FANATIC_OF_RHONAS;
        let mut token = baylee_client_core::test_support::token(536, 0, "Fanatic of Rhonas", 4, 4);
        token.base_power = Some(4);
        token.base_toughness = Some(4);
        token.rules = Some(RulesFace {
            card: fanatic,
            face: 0,
        });
        assert_eq!(printed_body(&token), Some((1, 4)), "the box the print says");
        let corner = Corner::of_object(&token);
        assert!(
            !corner.shows_plate(true, false),
            "this test's premise: against its base the plate hid"
        );
        assert!(
            corner
                .over_print(printed_body(&token))
                .shows_plate(true, false),
            "a 4/4 drawn on a 1/4 print wears its plate"
        );
        assert!(
            crate::platemat::words_of(&token).is_some(),
            "and the preview's"
        );

        // The card itself, on its own print, says its body there.
        let mut card = baylee_client_core::test_support::token(537, 0, "Fanatic of Rhonas", 1, 4);
        card.token = None;
        card.base_power = Some(1);
        card.base_toughness = Some(4);
        card.card = Some(baylee_view::CardIdentity {
            index: fanatic,
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        });
        card.rules = token.rules;
        assert_eq!(printed_body(&card), Some((1, 4)));
        assert!(
            crate::platemat::words_of(&card).is_none(),
            "a vanilla body is not written"
        );
    }
}
