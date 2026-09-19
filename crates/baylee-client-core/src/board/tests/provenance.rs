//! What a permanent is and whose picture it is wearing: the one judgement `worn` makes, read as the art, as the `Provenance` mark, and as the card left underneath a copy. No field in the view says "this is a copy", so every test here is built on a disagreement between the projected name and the object's own cardboard, and every one of them carries its counter-arm — a client with no registry to ask must find no copies and leave each permanent drawn as itself, which is the state the client was in before any of this existed. The asymmetry between the two lookups is here too: a card is compared by index and a registry token asked for by id, because two tokens share the name "Shapeshifter" and a name would call one of them a copy of its twin. Whether a permanent merged into a group is `lanes`; the size a key is asked at and the URL it resolves to are `contracts`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn tokens_have_no_art_key_and_are_marked_as_tokens() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![token(1, 0, "Soldier", 1, 1)])
        .build();
    let m = model(&view);
    let group = &m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane")
        .groups[0];
    assert_eq!(group.provenance, Provenance::Token);
    assert!(group.art.is_none());
    assert!(m.required_images().is_empty());
}

#[test]
fn a_registry_token_wears_its_own_picture_and_a_copy_token_the_card_it_copies() {
    // Two permanents with no printing between them, drawn from opposite
    // ends. A Soldier the registry knows carries the token id its art is
    // keyed on. A token some clone effect made carries neither that nor a
    // card — a copy of a *card* is on nobody's token list — so the only
    // handle it has ever had is the name it projects, and until the
    // registry was asked about that name it fell back to a coloured
    // rectangle with the name written across it.
    let mut soldier = token(1, 0, "Soldier", 1, 1);
    soldier.token = Some(8);
    let bear_token = token(2, 0, "Bear", 2, 2);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![soldier, bear_token])
        .build();
    let bear = CardIndex::new(7);
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        &[],
        Registry::of(&|name: &str| (name == "Bear").then_some(Wears::Card(bear, 0))),
    );
    let art = |m: &BoardModel, name: &str| {
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .find(|g| g.name == name)
            .expect("group")
            .art
    };
    assert_eq!(
        art(&m, "Soldier"),
        Some(ImageKey::token(8, ArtSize::Small)),
        "a registry token knows which picture it wears"
    );
    assert_eq!(
        art(&m, "Bear"),
        Some(ImageKey::card(bear, 0, ArtSize::Small)),
        "a copy token is drawn as the card it copies"
    );
    for key in [
        ImageKey::token(8, ArtSize::Small),
        ImageKey::card(bear, 0, ArtSize::Small),
    ] {
        assert!(
            m.required_images().contains(&key),
            "{key:?} has to be asked for, or nothing fetches it"
        );
    }

    // The counter-test, and the state every client that has no registry
    // to ask is in: a lookup that answers nothing leaves the copy token
    // exactly where it was — its own face with its own name on it, never
    // somebody else's picture.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &[], Registry::none());
    assert_eq!(art(&blind, "Bear"), None);
    assert_eq!(
        art(&blind, "Soldier"),
        Some(ImageKey::token(8, ArtSize::Small)),
        "and a registry token never needed the lookup"
    );
}

#[test]
fn a_permanent_that_has_become_a_copy_is_drawn_as_the_card_it_copies() {
    // The view says two things at once and only the second is what the
    // player is looking at. `card` is the cardboard — a copy effect
    // assigns characteristics and never a printing (CR 707.2 lists the
    // copiable values; CR 109.3 lists the characteristics, and neither
    // has art in it) — while `name` is the projection. So a Clone that
    // has become a Llanowar Elves was drawn as a Clone with "Llanowar
    // Elves" written under it, which is a card that does not exist.
    //
    // Object 3 is the Clone: its own card index 5, projecting the Elves'
    // name. Object 4 is a real Llanowar Elves, index 9, and it is the
    // control — the registry answers *itself* for it, so the disagreement
    // that marks a copy is absent and it keeps its own printing.
    let clone = printed(3, 0, "Llanowar Elves", 5);
    let real = printed(4, 0, "Llanowar Elves", 9);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![clone, real])
        .build();
    let elves = CardIndex::new(9);
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        &[],
        Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
    );
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    // Two groups, not one: `ObjectSummaryKey` carries the card, so a
    // Clone wearing another card's face can never be counted into a stack
    // with the card itself — which is what would have hidden the copy the
    // moment the two stood side by side.
    assert_eq!(lane.groups.len(), 2, "the copy and the card it copies");
    let of = |id: u32| {
        lane.groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
            .art
    };
    assert_eq!(
        of(3),
        Some(ImageKey::card(elves, 0, ArtSize::Small)),
        "the copy wears the picture of the card it copies"
    );
    assert_eq!(
        of(4),
        Some(ImageKey::new(PrintRef::new(9), 0, ArtSize::Small)),
        "and the card itself keeps the printing at this table"
    );

    // The counter-test: with no registry to ask, both fall back to their
    // own printings and the Clone is once again drawn as a Clone.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &[], Registry::none());
    let blind_lane = blind
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    assert_eq!(
        blind_lane
            .groups
            .iter()
            .find(|g| g.representative == ObjectId::new(3, 0))
            .expect("group")
            .art,
        Some(ImageKey::new(PrintRef::new(5), 0, ArtSize::Small))
    );
}

/// The same disagreement, read as the mark rather than as the picture.
///
/// Four permanents, one of each answer, in one board — because the three
/// arms are a chain and a test that asked them one at a time would not
/// notice an arm swallowing the case below it. The face-down one is the
/// arm that costs something to get right: its `card` is `None` for the
/// same reason a token's is, and reading only that field marks every
/// opponent's morph as a token.
#[test]
fn a_permanent_says_whether_it_is_a_token_a_copy_or_the_card_it_looks_like() {
    let elves = CardIndex::new(9);
    let mut face_down = printed(6, 0, "Face-down", 12);
    face_down.card = None;
    face_down.status = ObjectStatus::FACE_DOWN;
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![
                printed(3, 0, "Llanowar Elves", 5),  // a Clone
                printed(4, 0, "Llanowar Elves", 9),  // the card itself
                token(5, 0, "Llanowar Elves", 1, 1), // a copy token
                face_down,
            ],
        )
        .build();
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        &[],
        Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
    );
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let of = |id: u32| {
        lane.groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
            .provenance
    };
    assert_eq!(of(3), Provenance::Copy, "cardboard belonging to a Clone");
    assert_eq!(of(4), Provenance::Printed, "the card it looks like");
    // A token a copy effect made is a token and not a copy: there is no
    // original to go and look at, so a copy mark would promise one.
    assert_eq!(of(5), Provenance::Token, "a chit, whatever is drawn on it");
    assert_eq!(
        of(6),
        Provenance::Printed,
        "a morph is not a token, however alike the two look in the view"
    );

    // The counter-test. A client with nothing to ask still knows a token
    // from a card — that half needs no registry — and simply never finds
    // a copy, which is what it did before any of this existed.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &[], Registry::none());
    let blind_lane = blind
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let blind_of = |id: u32| {
        blind_lane
            .groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
            .provenance
    };
    assert_eq!(blind_of(3), Provenance::Printed);
    assert_eq!(blind_of(5), Provenance::Token);
}

/// The card underneath a copy is offered beside the one it is wearing.
///
/// Two claims, and the second is the one that would rot quietly: the key
/// is the *physical* printing rather than the worn card, and the board
/// asks for it to be resident — a preview opens on a hover and has no
/// frame to spend on a fetch.
#[test]
fn a_copy_offers_the_card_underneath_it() {
    let elves = CardIndex::new(9);
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![
                printed(3, 0, "Llanowar Elves", 5),  // a Clone
                printed(4, 0, "Llanowar Elves", 9),  // the card itself
                token(5, 0, "Llanowar Elves", 1, 1), // a copy token
            ],
        )
        .build();
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        &[],
        Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
    );
    let lane = m
        .pod(PlayerId::new(0))
        .and_then(|p| p.lane(LaneKind::Creatures))
        .expect("lane");
    let group = |id: u32| {
        lane.groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
    };
    let cardboard = ImageKey::new(PrintRef::new(5), 0, ArtSize::Small);
    assert_eq!(group(3).original, Some(cardboard), "the Clone's own print");
    assert_eq!(group(4).original, None, "a card stands in for nothing");
    assert_eq!(group(5).original, None, "a chit has nothing underneath it");
    // A second key beside the first and never a replacement for it: what
    // the table draws is still the card the copy is wearing.
    assert_eq!(
        group(3).art,
        Some(ImageKey::card(elves, 0, ArtSize::Small)),
        "the copy is still drawn as what it copies"
    );
    assert!(
        m.required_images().contains(&cardboard),
        "the hover would have to fetch it"
    );

    // The counter-arm. A client with nothing to ask finds no copies, so
    // no card on its board carries a second picture at all.
    let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, &[], Registry::none());
    assert!(
        blind
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .all(|g| g.original.is_none())
    );
}

/// A permanent copying a *token* is a copy, and is drawn as the chit.
///
/// The case entry 16 of `docs/observed-faults.md` was still getting
/// wrong. Both the mark and the picture were found by handing the
/// projected name to a lookup that only knew cards, so a Clone on a
/// Soldier answered the same `None` a permanent copying nothing answers:
/// no mark, no card underneath, and a Clone on the table with "Soldier"
/// written under it.
#[test]
fn a_permanent_copying_a_token_wears_the_token() {
    const SOLDIER: u16 = 9;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(3, 0, "Soldier", 5)])
        .build();
    let named = |name: &str| (name == "Soldier").then_some(Wears::Token(SOLDIER));
    let token_name = |id: u16| (id == SOLDIER).then_some("Soldier");
    let group = |m: &BoardModel| {
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups[0]
            .clone()
    };

    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        &[],
        Registry {
            named: &named,
            token_name: &token_name,
        },
    );
    let g = group(&m);
    assert_eq!(g.provenance, Provenance::Copy, "it is copying something");
    assert_eq!(
        g.art,
        Some(ImageKey::token(SOLDIER, ArtSize::Small)),
        "and what it is copying is a chit, which has its own picture"
    );
    let cardboard = ImageKey::new(PrintRef::new(5), 0, ArtSize::Small);
    assert_eq!(g.original, Some(cardboard), "the Clone is still underneath");
    assert!(m.required_images().contains(&cardboard));

    // The counter-arm, and it is the bug this test is about: a registry
    // that cannot name the token draws the Clone as itself and says
    // nothing is unusual.
    let blind = group(&BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        &[],
        Registry::none(),
    ));
    assert_eq!(blind.provenance, Provenance::Printed);
    assert_eq!(blind.art, Some(cardboard));
    assert_eq!(blind.original, None);
}

/// A token is never a copy of its own twin.
///
/// Two tokens in the registry are printed "Shapeshifter", so a name
/// answers with the first of them and an identity test made of names
/// would call the second a copy of the first — and draw the 2/2 as the
/// 1/1. A token's own name is asked for by *id* for exactly this reason,
/// which is the one asymmetry between the two arms of [`worn`].
#[test]
fn a_token_is_not_a_copy_of_the_twin_that_shares_its_name() {
    const ONE_ONE: u16 = 6;
    const TWO_TWO: u16 = 7;
    let mut chit = token(3, 0, "Shapeshifter", 2, 2);
    chit.token = Some(TWO_TWO);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![chit, printed(4, 0, "Shapeshifter", 5)])
        .build();
    let named = |name: &str| (name == "Shapeshifter").then_some(Wears::Token(ONE_ONE));
    let token_name = |id: u16| (id == ONE_ONE || id == TWO_TWO).then_some("Shapeshifter");
    let m = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| WIDE,
        &[],
        Registry {
            named: &named,
            token_name: &token_name,
        },
    );
    let group = |id: u32| {
        m.pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups
            .iter()
            .find(|g| g.representative == ObjectId::new(id, 0))
            .expect("group")
    };
    assert_eq!(group(3).provenance, Provenance::Token);
    assert_eq!(
        group(3).art,
        Some(ImageKey::token(TWO_TWO, ArtSize::Small)),
        "the 2/2 is drawn from its own id and not from its name"
    );
    assert_eq!(group(3).original, None, "a chit has nothing underneath it");

    // And the tie the other way round, which is the documented one: a
    // *card* copying either Shapeshifter is drawn as the first of them,
    // because a projected name is all there is to go on and two tokens
    // with one name are one name.
    assert_eq!(group(4).provenance, Provenance::Copy);
    assert_eq!(group(4).art, Some(ImageKey::token(ONE_ONE, ArtSize::Small)));
}
