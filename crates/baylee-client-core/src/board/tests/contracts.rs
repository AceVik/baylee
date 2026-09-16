//! Where this model is held against code it does not own. The keyword bits are a mirror of `KeywordSet`, so a renumbering there fails here instead of quietly redrawing every icon in the game; the image keys are what `crate::images` has to turn into a fetchable URL, asked for once each and at the one size the board draws at; and one view must always produce one scene, which is what lets a renderer diff a frame against the last. A test belongs here when what it pins is that boundary. A question about the picture of one particular thing — a copy, a fanned card, an ability on the stack — belongs with that thing and not here.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn keyword_bits_match_the_card_dsl() {
    // If the DSL renumbers a keyword, this fails rather than silently
    // drawing the wrong icon on every card in the game.
    use KeywordSet as K;
    assert_eq!(keyword_bits::FLYING, K::FLYING.bits());
    assert_eq!(keyword_bits::DEATHTOUCH, K::DEATHTOUCH.bits());
    assert_eq!(keyword_bits::TRAMPLE, K::TRAMPLE.bits());
    assert_eq!(keyword_bits::VIGILANCE, K::VIGILANCE.bits());
    assert_eq!(keyword_bits::DEFENDER, K::DEFENDER.bits());
    assert_eq!(keyword_bits::INDESTRUCTIBLE, K::INDESTRUCTIBLE.bits());
}

#[test]
fn one_card_in_three_places_is_one_image() {
    let mut card = token(1, 0, "Serra Angel", 4, 4);
    card.card = Some(CardIdentity {
        index: CardIndex::new(7),
        print: PrintRef::new(3),
        face: 0,
    });
    let mut same = card.clone();
    same.id = ObjectId::new(2, 0);

    let mut on_stack = card.clone();
    on_stack.id = ObjectId::new(3, 0);

    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![card, same])
        .with_stack(vec![on_stack])
        .build();
    let m = model(&view);
    let keys = m.required_images();

    // Three objects, one image: the two battlefield copies group, and the
    // stack copy asks at the same size they do. It used to ask at
    // `Normal`, which made this two entries — and meant a spell cast from
    // a hand the player could already see fetched its art a second time
    // and drew the constructed face until it landed. The board has exactly
    // one size now; the hover preview is the only thing that reads bigger.
    assert_eq!(keys.len(), 1);
    assert!(keys.iter().all(|k| k.size == ArtSize::Small));
}

#[test]
fn a_board_resolves_end_to_end_into_fetchable_image_urls() {
    use crate::images::resolve;
    use crate::test_support::{printed, statics};

    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Serra Angel", 4)])
        .build();
    let m = model(&view);
    let table = statics(8);

    let keys = m.required_images();
    assert_eq!(keys.len(), 1);
    let request =
        resolve(&table, keys[0], |_| None, |_| None).expect("the print table resolves it");
    assert!(
        request
            .url
            .starts_with("https://cards.scryfall.io/small/front/")
    );
    assert!(
        std::path::Path::new(&request.url)
            .extension()
            .is_some_and(|e| e == "jpg")
    );
}

#[test]
fn the_model_is_deterministic_for_a_given_view() {
    let objs: Vec<PublicObject> = (0..20)
        .map(|i| token(i, 0, if i % 2 == 0 { "A" } else { "B" }, 1, 1))
        .collect();
    let view = ViewBuilder::new(4).with_battlefield(0, objs).build();
    let a = model(&view);
    let b = model(&view);
    assert_eq!(a, b, "the same view must always produce the same scene");
}
