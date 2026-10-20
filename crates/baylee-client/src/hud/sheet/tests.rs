use super::*;

#[test]
fn a_single_armed_ability_keeps_its_printed_sentence_on_the_sheet() {
    use baylee_client_core::test_support::{ViewBuilder, printed};
    use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
    let card = baylee_cards::decks::by_name("Maskwood Nexus").unwrap();
    let mut object = printed(1, 0, "Maskwood Nexus", 1);
    object.card.as_mut().unwrap().index = card;
    object.rules = Some(baylee_view::RulesFace { card, face: 0 });
    let id = object.id;
    let mut duel = Duel::default();
    duel.receive_view(ViewBuilder::new(2).with_battlefield(0, [object]).build());
    duel.receive_choice(Pending::Priority {
        player: baylee_core::ids::PlayerId::new(0),
        legal: Box::new(LegalActions {
            abilities: vec![(id, 1)],
            ..Default::default()
        }),
    });
    duel.ability_menu = Some(id);
    duel.armed = Some(crate::Armed {
        object: id,
        deed: crate::Deed::Ability(PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        }),
    });
    let opening = ability_opening(&duel, Lang::En, &crate::cardtext::CardTexts::default())
        .expect("one costly ability still has a sheet");
    assert_eq!(opening.rows.len(), 1);
    assert_eq!(opening.armed, Some(0));
    assert!(
        opening.rows[0]
            .blocks
            .as_ref()
            .is_some_and(|b| !b.is_empty())
    );
}

#[test]
fn personal_incarnation_keeps_full_oracle_in_its_dialog_and_a_short_confirmation() {
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
    let mut object = crate::registry_printed(1, 1, "Personal Incarnation");
    object.owner = baylee_core::ids::PlayerId::new(0);
    let id = object.id;
    let card = object.rules.unwrap().card;
    let historical = "Der Besitzer der Eigenen Inkarnation kann Teile oder den ganzen Schaden, den der Inkarnation zugefügt wird, auf sich selber umleiten.\nWenn die Eigene Inkarnation aus dem Spiel auf einem Friedhof landet, verliert ihr Besitzer die (aufgerundete) Hälfte seiner Lebenspunkte.";
    let texts = crate::cardtext::CardTexts::filed(crate::cardtext::fixture::german(
        "Personal Incarnation",
        "Eigene Inkarnation",
        Some(historical),
    ));
    let mut duel = Duel::default();
    duel.receive_view(ViewBuilder::new(2).with_battlefield(1, [object]).build());
    duel.receive_choice(Pending::Priority {
        player: baylee_core::ids::PlayerId::new(0),
        legal: Box::new(LegalActions {
            abilities: vec![(id, 0)],
            ..LegalActions::default()
        }),
    });
    duel.ability_menu = Some(id);
    duel.armed = Some(crate::Armed {
        object: id,
        deed: crate::Deed::Ability(PlayerAction::ActivateAbility {
            source: id,
            ability_index: 0,
        }),
    });
    let opening = ability_opening(&duel, Lang::De, &texts).expect("the dialog remains open");
    assert_eq!(opening.rows.len(), 1);
    let sentence = opening.rows[0]
        .blocks
        .as_ref()
        .unwrap()
        .iter()
        .map(baylee_client_core::card_face::TextBlock::text)
        .collect::<Vec<_>>()
        .join(" ");
    // The printed `{0}` is the row's cost column, so the words follow it.
    let oracle = baylee_cards::oracle::sentence(card, 0, 0).unwrap();
    assert_eq!(Some(sentence.as_str()), oracle.strip_prefix("{0}: "));
    let words =
        crate::hud::overlay::armed_label(&duel, Lang::De, duel.armed.as_ref().unwrap()).unwrap();
    assert_eq!(words.text, "Fähigkeit aktivieren");
    assert_eq!(words.cost, None);
    assert_eq!(duel.ability_menu, Some(id));
    let at = baylee_view::StackText {
        face: 0,
        line: 0,
        of: 2,
    };
    assert_eq!(
        crate::cardtext::said(Some(&texts), card, at).unwrap().1,
        crate::cardtext::Said::Oracle
    );
}

/// A duel's window, measured off the running client.
const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);
/// A permanent on a duel's battlefield is about this many logical pixels
/// wide, and the box keeps the 63:88 card aspect.
const CARD: Vec2 = Vec2::new(47.0, 65.6);
/// A sheet of about five rows, at the width a middling sentence asks for.
const SHEET: Vec2 = Vec2::new(300.0, 210.0);

#[test]
fn the_sheet_sits_over_the_card_it_belongs_to() {
    let mid = Vec2::new(864.0, 600.0);
    let at = corner_for(mid, CARD, SHEET, WINDOW);
    assert!(
        (at.x + SHEET.x / 2.0 - mid.x).abs() < 0.5,
        "centred on the card: {at} against {mid}"
    );
    assert!(
        at.y + SHEET.y <= mid.y - CARD.y / 2.0,
        "and clear of its top edge: the sheet ends at {} and the card \
             starts at {}",
        at.y + SHEET.y,
        mid.y - CARD.y / 2.0
    );
}

/// The sheet is as wide as its own text now, so every width has to land
/// on the same card. A constant here would put a narrow sheet off-centre
/// by half the difference and nothing but the eye would catch it.
#[test]
fn a_sheet_of_any_width_is_centred_on_its_card() {
    let mid = Vec2::new(864.0, 600.0);
    for w in [SHEET_MIN, 260.0, 300.0, SHEET_MAX] {
        let at = corner_for(mid, CARD, Vec2::new(w, 210.0), WINDOW);
        assert!(
            (at.x + w / 2.0 - mid.x).abs() < 0.5,
            "a {w}-wide sheet sits at {at}, off the card's {}",
            mid.x
        );
    }
}

/// A permanent near the top of the window — an opponent's board — has no
/// room above it, and the sheet goes under the card rather than off the
/// screen.
#[test]
fn a_sheet_with_no_room_above_the_card_goes_below_it() {
    let mid = Vec2::new(864.0, 90.0);
    let at = corner_for(mid, CARD, SHEET, WINDOW);
    assert!(
        at.y >= mid.y + CARD.y / 2.0,
        "below the card: {} against {}",
        at.y,
        mid.y + CARD.y / 2.0
    );
    assert!(at.y + SHEET.y <= WINDOW.y, "and still inside the window");
}

/// Neither edge of the window may cut a row off. A card in the corner is
/// the ordinary case, not a contrived one: a seat's lands sit at the end
/// of a lane.
#[test]
fn a_sheet_beside_a_card_at_the_edge_stays_inside_the_window() {
    for x in [0.0, 20.0, 1700.0, WINDOW.x] {
        for w in [SHEET_MIN, SHEET_MAX] {
            let sheet = Vec2::new(w, 210.0);
            let at = corner_for(Vec2::new(x, 600.0), CARD, sheet, WINDOW);
            assert!(
                at.x >= SHEET_MARGIN && at.x + w <= WINDOW.x - SHEET_MARGIN,
                "at x={x} a {w}-wide sheet's corner is {at}"
            );
        }
    }
}

/// A sheet taller than the window it is drawn in still starts inside it.
/// Nine rows of wrapped rules text on a short window is the shape, and
/// the alternative is a header scrolled off the top edge.
#[test]
fn a_sheet_taller_than_the_window_still_starts_inside_it() {
    let at = corner_for(
        Vec2::new(400.0, 300.0),
        CARD,
        Vec2::new(300.0, 900.0),
        Vec2::new(900.0, 500.0),
    );
    assert!(at.y >= SHEET_MARGIN, "the top is on screen: {at}");
    assert!(at.x >= SHEET_MARGIN, "and so is the left edge");
}

/// A sheet opened from a card at the top-right of the window stands
/// clear of the report button (#309), which is over everything there: a
/// sheet under it would lose its head, which names the card.
#[test]
fn a_sheet_never_opens_under_the_report_button() {
    let corner = report_corner(WINDOW);
    for x in [WINDOW.x - 20.0, WINDOW.x - 120.0, WINDOW.x - 200.0] {
        for y in [20.0, 60.0, 120.0, 280.0, 290.0, 300.0] {
            let at = corner_for(Vec2::new(x, y), CARD, SHEET, WINDOW);
            let sheet = Rect::from_corners(at, at + SHEET);
            assert!(
                sheet.intersect(corner).is_empty(),
                "a card at ({x}, {y}) opens its sheet at {at}, under the button {corner:?}"
            );
        }
    }
}

/// A bubble is a sheet two lines high and goes where a sheet goes. It
/// spent one commit opening *below* the card, which is where the seat's
/// own phase rail is drawn for a card in the land lane.
#[test]
fn a_mana_bubble_opens_over_the_card_like_any_other_sheet() {
    let mid = Vec2::new(864.0, 600.0);
    let bubble = Vec2::new(150.0, 42.0);
    let at = corner_for(mid, CARD, bubble, WINDOW);
    assert!(
        at.y + bubble.y <= mid.y - CARD.y / 2.0,
        "above the card's top edge: the bubble ends at {} and the card \
             starts at {}",
        at.y + bubble.y,
        mid.y - CARD.y / 2.0
    );
}
