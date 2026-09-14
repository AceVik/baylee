use super::*;
// The registry here is the empty one, and deliberately: a zone browser
// lists cards in hidden zones, and a card that arrives in one is a new
// object with no memory of its previous existence (CR 400.7), so whatever
// it was copying on the battlefield it is not copying in a graveyard.
// Nothing a browser draws is ever wearing another card's face.
use crate::board::{BoardModel, Openings, Registry};
use crate::test_support::{ViewBuilder, printed};
use baylee_engine::choice::{ChoicePrompt, Pending, TargetPrompt};

fn me() -> PlayerId {
    PlayerId::new(0)
}

/// What is ticked, in tab order, as a list a test can read.
///
/// A `Vec` and not the set itself: the empty case is what "Alle" means
/// and `vec![]` says that in the assertion, where `BTreeSet::new()` says
/// only that the set is empty.
fn ticks(b: &Browser) -> Vec<BrowseZone> {
    b.ticked().iter().copied().collect()
}

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// The grid never draws a card wider than the one size the art has.
///
/// The three measures are the sheet's own: its floor, its default and a
/// maximised sheet on this screen, each less the gutters the list keeps.
#[test]
fn a_tile_grows_into_the_gaps_and_never_past_the_art() {
    // 73 is one texel to one pixel at scale 2; 100 is where the softening
    // starts to show. `hud::tray` owns both numbers and states why.
    let (min, max, gap) = (73.0, 100.0, 10.0);
    for measure in [324.0_f32, 670.0, 1400.0, 2976.0] {
        let (across, each, air) = grid_across(measure, min, max, gap);
        assert!(across >= 1, "{measure} px fits no tile at all");
        assert!(
            (min..=max).contains(&each),
            "{measure} px draws a {each}-wide tile, which is outside the art's own size"
        );
        assert!(air >= gap, "{measure} px squeezes the gap to {air}");
        #[allow(clippy::cast_precision_loss)]
        let used = across as f32 * each + (across - 1) as f32 * air;
        assert!(
            used <= measure + 0.01,
            "{across} tiles of {each} with {air} between them is {used}, wider than {measure}"
        );
    }
}

/// And when the cap does bite, the leftover goes into the air rather than
/// into the picture.
///
/// It bites on a *narrow* measure and not on a wide one, which is the
/// part that is easy to have backwards: the count is taken at `min`
/// first, so a wide measure is a row of many tiles each barely above the
/// floor. Only two or three columns leave a share big enough to reach the
/// cap — and `hud::tray`'s own test is the other half of this, showing
/// that the browser's floor width fits four and so never gets here.
#[test]
fn a_capped_row_widens_its_gaps_instead_of_its_cards() {
    let (across, each, air) = grid_across(368.0, 73.0, 100.0, 10.0);
    assert_eq!(across, 4, "368 px fits four tiles at the floor");
    assert!(
        (each - 84.5).abs() < 0.01,
        "four across 368 px is 84.5 each, not {each}"
    );
    assert!((air - 10.0).abs() < f32::EPSILON, "uncapped keeps its gap");

    let (across, each, air) = grid_across(230.0, 73.0, 100.0, 10.0);
    assert_eq!(across, 2, "230 px fits two tiles and not a third");
    assert!(
        (each - 100.0).abs() < 0.01,
        "two across 230 px hits the cap"
    );
    assert!(
        (air - 30.0).abs() < 0.01,
        "the 30 px the cards gave up did not go into the gap; it is {air}"
    );
    assert!(
        (2.0f32.mul_add(each, air) - 230.0).abs() < 0.01,
        "a capped row still fills its measure"
    );
}

/// A stored view mode this build does not know is the default, not a
/// refusal that takes the whole settings file with it.
#[test]
fn an_unknown_view_mode_reads_as_the_default() {
    for mode in ViewMode::ALL {
        let text = serde_json::to_string(&mode).expect("a mode writes");
        let back: ViewMode = serde_json::from_str(&text).expect("and reads");
        assert_eq!(back, mode, "{} did not survive the round trip", mode.name());
    }
    let retired: ViewMode = serde_json::from_str("\"folders\"").expect("an unknown name reads");
    assert_eq!(
        retired,
        ViewMode::default(),
        "a mode this build has never heard of has to fall back, not fail: the client's \
         settings store answers a refusal with Default and would take the player's sheet, \
         their language and their address down with it"
    );
}

/// Everything a client can already click without the browser: the
/// battlefield as drawn cards, and the seat's own hand.
fn drawn_on_the_table(view: &PlayerView) -> Vec<ObjectId> {
    let board = BoardModel::from_view(view, Openings::none(), |_| 100.0, Registry::none());
    let mut ids: Vec<ObjectId> = board
        .pods
        .iter()
        .flat_map(|p| p.lanes.iter())
        .flat_map(|l| l.groups.iter())
        .flat_map(|g| g.members.iter().copied())
        .collect();
    ids.extend(board.hand.iter().map(|c| c.id));
    ids
}

#[test]
fn a_library_search_is_shown_where_the_board_cannot_show_it() {
    // Four cards the engine is *showing* the seat. They are in nobody's
    // graveyard and on no battlefield, so before the browser existed the
    // only thing on screen was the prompt.
    let shown: Vec<_> = (10..14).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2).with_looking_at(shown).build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..14).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );

    assert!(Browser::wanted(&view, &it), "nothing else can draw these");
    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(b.is_open());

    let rows = b.rows(&view, Some(&it), Names::projected());
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.zone == BrowseZone::Looking));
    assert!(
        rows.iter().all(|r| r.standing.selectable),
        "all four were offered"
    );
    assert!(rows.iter().all(|r| r.art.is_some()), "each has a picture");
}

/// W1, as the owner measured it: the sheet sat 164 pixels left of the
/// middle of a 2056-pixel window, at exactly the place it would be
/// centred in a 1728-pixel one.
///
/// Nobody had dragged it there — it was remembered from a smaller screen,
/// and `fit` has no opinion about the middle of a band. So a question's
/// sheet reads no store at all, and the two window widths have to answer
/// the same word: centred.
#[test]
fn a_sheet_a_question_opened_is_centred_on_whatever_window_it_meets() {
    let shown: Vec<_> = (10..14).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2).with_looking_at(shown).build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..14).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        me(),
    );

    // What the store held: the sheet centred on the smaller window.
    //
    // That is how the fault was found. The owner measured the dialog at
    // `x ≈ 437..1290` on a 2056-pixel window, and 437 is exactly
    // `(1728 - 854) / 2` — the sheet's width at the time, centred in a
    // band 1728 wide. So it had been centred, on a *different* screen,
    // and remembered; `fit` clamps a rectangle inside a band and has no
    // opinion about the middle of one.
    let small = (1728.0, 776.0);
    let remembered = Placement::centred(small);
    assert!(
        (remembered.left - (small.0 - Placement::DEFAULT_W) / 2.0).abs() < 1.0,
        "the arithmetic that found the fault"
    );

    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(b.for_choice(), "a question is what opened it");

    let middle = |place: Placement| place.left + place.width / 2.0;
    for band in [small, (2056.0, 776.0)] {
        let place = b.placement(band, Some(remembered));
        assert!(
            (middle(place) - band.0 / 2.0).abs() < 1.0,
            "a question's sheet is centred in {band:?}, not where a smaller window left it"
        );
    }

    // The other half of the same rule: a sheet the *player* opened is
    // where they put it, because that is what dragging it means.
    let mut by_hand = Browser::new();
    by_hand.open_at(BrowseZone::Looking);
    let held = by_hand.placement((2056.0, 776.0), Some(remembered));
    assert!(
        (held.left - remembered.left).abs() < 1.0,
        "a dragged sheet stays dragged"
    );
}

/// W3: a search is about the cards being shown and about nothing else,
/// so the tabs beside them are not part of the question.
#[test]
fn a_question_that_lives_in_one_zone_pins_the_tab_to_it() {
    let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
    let buried: Vec<_> = (20..22).map(|s| printed(s, 0, "Mountain", 1)).collect();
    let view = ViewBuilder::new(2)
        .with_looking_at(shown)
        .with_graveyard(0, buried)
        .build();
    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..13).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        me(),
    );

    let mut b = Browser::new();
    b.follow(&view, Some(&search));
    assert_eq!(b.locked(), Some(BrowseZone::Looking));
    assert_eq!(ticks(&b), vec![BrowseZone::Looking]);
    assert!(
        b.rows(&view, Some(&search), Names::projected())
            .iter()
            .all(|r| r.zone == BrowseZone::Looking),
        "the graveyard has nothing to answer here"
    );

    // The pin is the model's, not the renderer's: a click that reached
    // "every zone" anyway changes nothing.
    b.show(None);
    assert_eq!(ticks(&b), vec![BrowseZone::Looking], "the pin holds");

    // A question that reaches two zones pins nothing — there is no one
    // tab that could answer it.
    let across = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(10), obj(20)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    b.follow(&view, Some(&across));
    assert_eq!(b.locked(), None);
    assert_eq!(ticks(&b), vec![], "every zone at once");

    // And the pin belongs to the question: answered, the sheet the player
    // opens next is theirs to steer again.
    b.follow(&view, Some(&search));
    assert_eq!(b.locked(), Some(BrowseZone::Looking));
    b.close();
    assert_eq!(b.locked(), None);
    b.open();
    b.show(Some(BrowseZone::Graveyard(me())));
    assert_eq!(ticks(&b), vec![BrowseZone::Graveyard(me())]);
}

/// W2: the dim says "there is nothing else to do", so it is drawn only
/// when that is true — which is a narrower thing than "a question opened
/// this sheet".
#[test]
fn the_table_goes_dark_only_when_every_answer_is_in_the_sheet() {
    let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2)
        .with_looking_at(shown)
        .with_hand(vec![("Ornithopter", 0, 30)])
        .build();
    let mut b = Browser::new();
    assert!(!b.dims_the_table(), "a shut sheet darkens nothing");

    // A search: every card it offers is in the one pile it put on screen.
    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..13).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        me(),
    );
    b.follow(&view, Some(&search));
    assert!(b.dims_the_table(), "nothing outside the sheet is an answer");

    // A question that also offers a card in hand. The sheet still opens —
    // the revealed cards are nowhere else — but the hand is an answer,
    // and a veil over it would be darkening the thing to click.
    let spanning = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(10), obj(30)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    b.follow(&view, Some(&spanning));
    assert!(b.for_choice(), "a question is still what opened it");
    assert!(
        !b.dims_the_table(),
        "the hand holds an answer and must stay lit"
    );

    // And a pile the player opened to read stands over a live game.
    let mut by_hand = Browser::new();
    by_hand.open_at(BrowseZone::Graveyard(me()));
    assert!(!by_hand.dims_the_table(), "the game goes on underneath");
}

/// One question, one Confirm. The dialog's footer is where the answer is
/// sent from, and the prompt slip reads this same predicate to keep out of
/// its way.
#[test]
fn only_the_sheet_the_question_opened_draws_its_footer() {
    let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2)
        .with_looking_at(shown)
        .with_hand(vec![("Ornithopter", 0, 30)])
        .with_battlefield(0, [printed(40, 0, "Grizzly Bears", 2)])
        .with_graveyard(0, vec![printed(50, 0, "Lightning Bolt", 3)])
        .build();

    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..13).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        me(),
    );
    let mut b = Browser::new();
    assert!(
        !b.answers_here(Some(&search)),
        "a shut sheet answers nothing"
    );
    b.follow(&view, Some(&search));
    assert!(b.answers_here(Some(&search)), "this is the question's home");
    assert!(
        !b.answers_here(None),
        "and a sheet with no question left in it draws no footer either"
    );

    // A choice that spans the sheet and the hand: the table stays lit, and
    // the send still happens here, because there is nowhere else for it.
    let spanning = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(10), obj(30)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    b.follow(&view, Some(&spanning));
    assert!(!b.dims_the_table(), "the hand is an answer");
    assert!(
        b.answers_here(Some(&spanning)),
        "and this is still the send"
    );

    // The case the footer used to get wrong: a question about the
    // battlefield, and a graveyard the player opened to read while they
    // think about it. Nothing in that pile is an answer.
    let on_the_table = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(40)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    let mut by_hand = Browser::new();
    by_hand.open_at(BrowseZone::Graveyard(me()));
    by_hand.follow(&view, Some(&on_the_table));
    assert!(by_hand.is_open(), "the pile the player opened stays open");
    assert!(
        !by_hand.answers_here(Some(&on_the_table)),
        "the table is where that question is answered"
    );
}

/// The sheet is put where it fits, and shrunk before it is moved.
#[test]
fn a_sheet_is_brought_inside_the_band_it_stands_in() {
    let band = (1728.0, 776.0);
    let home = Placement::centred(band);
    assert!((home.width - Placement::DEFAULT_W).abs() < f32::EPSILON);
    assert!(
        (home.left - (band.0 - home.width) / 2.0).abs() < 0.01,
        "a sheet nobody has moved stands in the middle"
    );

    // A remembered position from a wider screen comes home rather than
    // hanging off the edge, taking its resize handle with it.
    let strayed = Placement {
        left: 3000.0,
        top: 2000.0,
        ..home
    };
    let back = strayed.fit(band);
    assert!(back.left + back.width <= band.0, "off the right edge");
    assert!(back.top + back.height <= band.1, "off the bottom edge");

    // Size before position: a sheet wider than the band is narrowed
    // first, so its own width cannot then push it off the far side.
    let huge = Placement {
        left: 900.0,
        top: 600.0,
        width: 5000.0,
        height: 5000.0,
    }
    .fit(band);
    assert!(huge.width < band.0 && huge.height < band.1);
    assert!(huge.left + huge.width <= band.0);
    assert!(huge.top + huge.height <= band.1);

    // And the minimum wins over a band with no room for it: a sheet
    // clamped to nothing is a sheet that is not there.
    let cramped = Placement::centred((120.0, 90.0));
    assert!((cramped.width - Placement::MIN_W).abs() < f32::EPSILON);
    assert!((cramped.height - Placement::MIN_H).abs() < f32::EPSILON);
}

/// Dragging moves it, the corner resizes it, and neither can leave the
/// band — which is what keeps the resize handle reachable.
#[test]
fn a_sheet_cannot_be_dragged_or_stretched_out_of_reach() {
    let band = (1728.0, 776.0);
    let home = Placement::centred(band);

    let nudged = home.moved_by((40.0, -25.0), band);
    assert!((nudged.left - (home.left + 40.0)).abs() < 0.01);
    assert!((nudged.top - (home.top - 25.0)).abs() < 0.01);
    assert!(
        (nudged.width - home.width).abs() < f32::EPSILON,
        "a drag is not a resize"
    );

    let shoved = home.moved_by((-9999.0, -9999.0), band);
    assert!(shoved.left >= 0.0 && shoved.top >= 0.0);

    let stretched = home.resized_by((60.0, 40.0), band);
    assert!((stretched.width - (home.width + 60.0)).abs() < 0.01);
    assert!(
        (stretched.left - home.left).abs() < f32::EPSILON,
        "the corner drags the corner, not the sheet"
    );

    let squashed = home.resized_by((-9999.0, -9999.0), band);
    assert!((squashed.width - Placement::MIN_W).abs() < f32::EPSILON);
    assert!((squashed.height - Placement::MIN_H).abs() < f32::EPSILON);
}

/// A reveal with no question attached opens the sheet by itself.
///
/// This is the job the "Zones" chip used to do and the reason the chip
/// could not simply be deleted: cards in `looking_at` are shown to a seat
/// without anything being asked of them, they are drawn on no other
/// surface in the client, and `follow` only ever runs when a *choice*
/// arrives. Edge-triggered on the ids, so the player can put it away.
/// A library has no tab, so a tap on one has nowhere to go.
///
/// The second of the two readings that enforce CR 401.2 — `ZonePile::
/// is_browsable` is the other — and the one that would silently start
/// working if a `Looking`-shaped variant were ever added for libraries.
#[test]
fn a_library_has_no_tab_to_open() {
    use crate::layout::PileKind;

    let seat = PlayerId::new(0);
    assert_eq!(BrowseZone::of_pile(PileKind::Library, seat), None);
    assert_eq!(
        BrowseZone::of_pile(PileKind::Graveyard, seat),
        Some(BrowseZone::Graveyard(seat))
    );
    assert_eq!(
        BrowseZone::of_pile(PileKind::Exile, seat),
        Some(BrowseZone::Exile(seat))
    );
    // Both command piles are one zone (CR 408.1): a seat with two
    // commanders has two places on the mat and one tab.
    assert_eq!(
        BrowseZone::of_pile(PileKind::Command2, seat),
        BrowseZone::of_pile(PileKind::Command, seat)
    );
}

#[test]
fn cards_shown_to_a_seat_open_the_sheet_by_themselves() {
    let mut b = Browser::new();
    let nothing = ViewBuilder::new(2).build();
    b.saw_reveal(&nothing);
    assert!(!b.is_open(), "an empty reveal is not a reveal");

    let shown = ViewBuilder::new(2)
        .with_looking_at(vec![printed(10, 0, "Ponder", 1)])
        .build();
    b.saw_reveal(&shown);
    assert!(
        b.is_open(),
        "cards being shown open the sheet that draws them"
    );
    assert_eq!(ticks(&b), vec![BrowseZone::Looking]);

    // …and it stays closed once the player closes it, however many views
    // arrive carrying the same cards. A per-frame decision would make the
    // panel impossible to dismiss.
    b.close();
    for _ in 0..5 {
        b.saw_reveal(&shown);
        assert!(!b.is_open(), "the same reveal re-opened it");
    }

    // A reveal that ends and another that begins is two reveals, and the
    // second one opens it again — which a length comparison would miss,
    // because both are one card.
    let other = ViewBuilder::new(2)
        .with_looking_at(vec![printed(11, 0, "Brainstorm", 2)])
        .build();
    b.saw_reveal(&other);
    assert!(b.is_open(), "a different reveal is a new one");

    // But the *same* cards in a different order are the same reveal. A
    // scry is exactly that — every rearrangement comes back as a view —
    // and a sheet the player had closed must not reappear on each one.
    let top = printed(20, 0, "Island", 3);
    let under = printed(21, 0, "Opt", 4);
    let ordered = ViewBuilder::new(2)
        .with_looking_at(vec![top.clone(), under.clone()])
        .build();
    let swapped = ViewBuilder::new(2)
        .with_looking_at(vec![under, top])
        .build();
    b.saw_reveal(&ordered);
    b.close();
    b.saw_reveal(&swapped);
    assert!(!b.is_open(), "reordering the same cards reopened the sheet");
}

/// The invariant the whole module exists for: an id the engine offered
/// is an id somebody draws. `BoardModel` covers the table and the hand,
/// `Browser` covers everything else, and the two are disjoint by
/// construction — which is why [`BrowseZone`] has no `Battlefield`.
#[test]
fn every_offered_object_is_drawn_somewhere() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_hand(vec![("Lightning Bolt", 1, 2)])
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_exile(1, vec![printed(5, 1, "Path to Exile", 4)])
        .with_command(0, vec![printed(6, 0, "Sisay", 5)])
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();

    let choices = [
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1), obj(3), obj(4), obj(5), obj(6), obj(7)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        Pending::ChooseCards {
            player: me(),
            options: vec![obj(4), obj(7)],
            min: 1,
            max: 2,
            prompt: ChoicePrompt::Generic,
        },
        Pending::LegendChoice {
            player: me(),
            options: vec![obj(1)],
        },
        Pending::OrderObjects {
            player: me(),
            objects: vec![obj(7), obj(4)],
        },
    ];

    let table = drawn_on_the_table(&view);
    for pending in choices {
        let it = Interaction::new(pending.clone(), me());
        let mut b = Browser::new();
        b.follow(&view, Some(&it));
        let rows = b.rows(&view, Some(&it), Names::projected());
        for id in it.selectable() {
            let on_table = table.contains(id);
            let in_tray = rows.iter().any(|r| r.id == *id && r.standing.selectable);
            assert!(
                on_table || in_tray,
                "{pending:?} offers {id:?} and nothing draws it"
            );
            assert!(
                !(on_table && in_tray),
                "{id:?} is drawn twice — the two models are meant to be disjoint"
            );
        }
    }
}

#[test]
fn a_choice_confined_to_the_table_leaves_the_tray_shut() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_hand(vec![("Lightning Bolt", 1, 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let it = Interaction::new(
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1), obj(2)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    );
    assert!(!Browser::wanted(&view, &it));
    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(
        !b.is_open(),
        "a target on the board is clicked on the board"
    );
}

/// A search that opened the sheet closes it again when it is answered.
///
/// The fetchland's round trip, which is what the owner asked for: the
/// library goes on screen, a land is picked, and the next question wants
/// nothing from the sheet — so the sheet gets out of the way instead of
/// standing over the board until somebody closes it.
#[test]
fn a_sheet_opened_for_a_search_shuts_when_the_search_is_answered() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: vec![obj(4)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    let mut b = Browser::new();
    b.follow(&view, Some(&search));
    assert!(b.is_open(), "the search did not open it");

    // The answer went in; the engine's next question is about the board.
    let after = Interaction::new(
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    );
    b.follow(&view, Some(&after));
    assert!(!b.is_open(), "the sheet stayed open with nothing to say");
}

/// But a sheet the *player* opened is never closed behind their back.
#[test]
fn a_sheet_opened_by_hand_survives_the_next_question() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let mut b = Browser::new();
    b.open_at(BrowseZone::Graveyard(me()));
    let it = Interaction::new(
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    );
    b.follow(&view, Some(&it));
    assert!(b.is_open(), "the player's own sheet was closed for them");
}

/// A reveal opens the sheet and the reveal ending closes it.
#[test]
fn a_reveal_takes_its_sheet_away_with_it() {
    let shown = ViewBuilder::new(2)
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();
    let done = ViewBuilder::new(2).build();
    let mut b = Browser::new();
    b.saw_reveal(&shown);
    assert!(b.is_open(), "the reveal did not open it");
    b.saw_reveal(&done);
    assert!(!b.is_open(), "the sheet outlived what it was showing");
}

/// The maximised sheet is the band filled, and it is a fixed point.
#[test]
fn a_maximised_sheet_fills_the_band_and_says_so() {
    let band = (1728.0, 866.0);
    let full = Placement::maximised(band);
    assert!(full.is_maximised(band));
    assert!(full.fit(band).is_maximised(band), "fitting it moved it");
    assert!(!Placement::centred(band).is_maximised(band));
    // And it stays inside: the margin is kept on all four sides.
    assert!(full.left + full.width <= band.0);
    assert!(full.top + full.height <= band.1);
}

#[test]
fn an_ordering_opens_the_tray_and_numbers_each_pick() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![
            printed(7, 0, "Ponder", 6),
            printed(8, 0, "Brainstorm", 7),
        ])
        .build();
    let mut it = Interaction::new(
        Pending::OrderObjects {
            player: me(),
            objects: vec![obj(7), obj(8)],
        },
        me(),
    );
    assert!(Browser::wanted(&view, &it), "an ordering always wants it");

    let b = Browser::new();
    assert!(
        b.rows(&view, Some(&it), Names::projected())
            .iter()
            .all(|r| r.place.is_none()),
        "nothing picked yet"
    );
    it.toggle(obj(8));
    it.toggle(obj(7));
    let rows = b.rows(&view, Some(&it), Names::projected());
    let place = |id| rows.iter().find(|r| r.id == id).and_then(|r| r.place);
    assert_eq!(place(obj(8)), Some(1), "picked first, so it goes first");
    assert_eq!(place(obj(7)), Some(2));
}

/// A discard leaves its options implicit — the engine means "your hand".
/// `is_selectable` says yes to anything for those, so a browser that
/// asked *that* question would offer every graveyard card as a discard.
#[test]
fn an_implicit_choice_does_not_light_up_the_whole_table() {
    let view = ViewBuilder::new(2)
        .with_hand(vec![("Lightning Bolt", 1, 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let it = Interaction::new(
        Pending::DiscardChoice {
            player: me(),
            count: 1,
        },
        me(),
    );
    assert!(it.is_selectable(obj(4)), "the interaction accepts anything");
    assert!(
        !Browser::wanted(&view, &it),
        "but the hand is already drawn"
    );
    let b = Browser::new();
    assert!(
        b.rows(&view, Some(&it), Names::projected())
            .iter()
            .all(|r| !r.standing.selectable),
        "a graveyard card is not a legal discard"
    );
}

#[test]
fn a_pile_can_be_read_with_no_question_pending() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
        .build();
    let mut b = Browser::new();
    b.open_at(BrowseZone::Graveyard(PlayerId::new(1)));

    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1, "the tab confines it to one pile");
    assert_eq!(rows[0].name, "Birds of Paradise");
    assert!(
        !rows[0].standing.selectable,
        "there is nothing to select for"
    );
    assert!(rows[0].place.is_none());

    b.show(None);
    assert_eq!(
        b.rows(&view, None, Names::projected()).len(),
        2,
        "both piles, unfiltered"
    );
}

/// The graveyard's own order is the default and is information: it is
/// what "the top card of your graveyard" means.
#[test]
fn the_default_order_is_the_pile_s_own_and_survives_being_reversed() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Zealous Persecution", 3),
                printed(5, 0, "Ancestral Vision", 4),
                printed(6, 0, "Mox Diamond", 5),
            ],
        )
        .build();
    let mut b = Browser::new();
    assert_eq!(b.sort(), SortKey::Place);
    let names = |b: &Browser| -> Vec<String> {
        b.rows(&view, None, Names::projected())
            .into_iter()
            .map(|r| r.name)
            .collect()
    };
    assert_eq!(
        names(&b),
        [
            "Zealous Persecution".to_string(),
            "Ancestral Vision".to_string(),
            "Mox Diamond".to_string()
        ],
        "the pile was re-ordered with no sort asked for"
    );

    b.reverse();
    assert_eq!(
        names(&b),
        [
            "Mox Diamond".to_string(),
            "Ancestral Vision".to_string(),
            "Zealous Persecution".to_string()
        ],
        "reversing the pile order did not reverse it"
    );
}

#[test]
fn sorting_by_name_and_by_cost_are_both_stable_and_reversible() {
    let mut cheap = printed(4, 0, "Zealous Persecution", 3);
    cheap.mana_value = 2;
    let mut dear = printed(5, 0, "Ancestral Vision", 4);
    dear.mana_value = 9;
    let mut also_cheap = printed(6, 0, "Mox Diamond", 5);
    also_cheap.mana_value = 2;
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![cheap, dear, also_cheap])
        .build();
    let mut b = Browser::new();
    let names = |b: &Browser| -> Vec<String> {
        b.rows(&view, None, Names::projected())
            .into_iter()
            .map(|r| r.name)
            .collect()
    };

    b.sort_by(SortKey::Name);
    assert_eq!(
        names(&b),
        [
            "Ancestral Vision".to_string(),
            "Mox Diamond".to_string(),
            "Zealous Persecution".to_string()
        ]
    );

    b.sort_by(SortKey::ManaValue);
    assert_eq!(
        names(&b),
        [
            // Two twos, and the tie is broken by the pile's own order —
            // never by whatever the previous sort happened to leave.
            "Zealous Persecution".to_string(),
            "Mox Diamond".to_string(),
            "Ancestral Vision".to_string()
        ]
    );
    b.reverse();
    assert_eq!(
        names(&b),
        [
            "Ancestral Vision".to_string(),
            "Zealous Persecution".to_string(),
            "Mox Diamond".to_string()
        ],
        "descending reversed the tie-break as well as the key"
    );
}

/// Whatever the key, the tabs are the panel's first structure.
#[test]
fn a_sort_never_interleaves_two_zones() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Mox Diamond", 3)])
        .with_exile(0, vec![printed(5, 0, "Ancestral Vision", 4)])
        .build();
    let mut b = Browser::new();
    b.sort_by(SortKey::Name);
    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].zone,
        BrowseZone::Graveyard(PlayerId::new(0)),
        "the exile card sorted ahead of the graveyard it is not in"
    );
    assert_eq!(rows[1].zone, BrowseZone::Exile(PlayerId::new(0)));
}

/// Where the keyboard is standing has to reach the row, or the key that
/// ticks it is ticking something the player cannot pick out of a list.
///
/// One row at a time, and never the chosen one by accident: `focused`
/// and `selected` are two different claims about the same row — the
/// client saying where a press would land, and the answer itself.
#[test]
fn the_row_the_keyboard_stands_on_says_so() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Llanowar Elves", 3),
                printed(5, 0, "Forest", 4),
            ],
        )
        .build();
    let mut it = Interaction::new(
        baylee_engine::choice::Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(4, 0), ObjectId::new(5, 0)],
            min: 0,
            max: 2,
            prompt: baylee_engine::choice::ChoicePrompt::Generic,
        },
        PlayerId::new(0),
    );
    let b = Browser::new();
    let focus_of = |it: &Interaction| -> Vec<bool> {
        b.rows(&view, Some(it), Names::projected())
            .iter()
            .map(|row| row.standing.focused)
            .collect()
    };
    assert_eq!(focus_of(&it), vec![true, false], "it starts on the first");
    it.cycle_focus(1);
    assert_eq!(focus_of(&it), vec![false, true], "and the walk moves it");
    // Ticking the second leaves the focus exactly where it was: one row
    // is chosen, the same row is focused, and they are still two flags.
    it.toggle_focused();
    let rows = b.rows(&view, Some(&it), Names::projected());
    assert_eq!(
        rows.iter()
            .map(|r| (r.standing.focused, r.standing.selected))
            .collect::<Vec<_>>(),
        vec![(false, false), (true, true)]
    );
}

/// A panel with no question in front of it stands on nothing.
#[test]
fn a_browse_with_no_question_focuses_no_row() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Forest", 3)])
        .build();
    let rows = Browser::new().rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].standing.focused);
}

/// A tap that lands on a pile is not a request to abandon the search.
///
/// `open_at` is the only door that writes `tab` without asking the lock,
/// and it also turns a `ForChoice` opening into a by-hand one — so a tap
/// on a graveyard while a library search stood open took the sheet away
/// from the question, `answers_here` went false, and the question's own
/// keys stopped working on a dialog that was still on the screen.
#[test]
fn a_tap_on_a_pile_does_not_take_the_sheet_from_a_question() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![printed(4, 0, "Forest", 3)])
        .build();
    let it = Interaction::new(
        baylee_engine::choice::Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(4, 0)],
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::Generic,
        },
        PlayerId::new(0),
    );
    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(b.answers_here(Some(&it)), "the sheet holds the question");

    b.open_at(BrowseZone::Graveyard(PlayerId::new(0)));

    assert!(
        b.answers_here(Some(&it)),
        "a tap on a pile took the sheet away from the question"
    );
    assert_eq!(
        ticks(&b),
        vec![BrowseZone::Looking],
        "and it must not have moved the tab either"
    );
}

/// The counter-test: with no question standing, a tap on a pile is
/// exactly what opens that pile, which is the whole job of `open_at`.
#[test]
fn a_tap_on_a_pile_still_opens_that_pile() {
    let mut b = Browser::new();
    b.open_at(BrowseZone::Graveyard(PlayerId::new(0)));
    assert!(b.is_open());
    assert_eq!(ticks(&b), vec![BrowseZone::Graveyard(PlayerId::new(0))]);
}

/// The cycle is one control, so a direction must not survive a key change.
#[test]
fn cycling_the_sort_key_starts_it_the_right_way_up() {
    let mut b = Browser::new();
    b.reverse();
    assert!(b.descending());
    b.cycle_sort();
    assert_eq!(b.sort(), SortKey::Name);
    assert!(!b.descending(), "descending carried into a new key");
}

#[test]
fn the_filter_narrows_by_name_and_ignores_case() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Llanowar Elves", 3),
                printed(5, 0, "Forest", 4),
            ],
        )
        .build();
    let mut b = Browser::new();
    b.set_filter("ELV");
    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Llanowar Elves");

    b.set_filter("  ");
    assert_eq!(
        b.rows(&view, None, Names::projected()).len(),
        2,
        "blank is not a filter"
    );
}

/// The panel drew one name and searched another.
///
/// A seat reading German sees *Wald* on the row — the renderer has
/// translated the drawn name since the catalog existed — and typing
/// `Wald` into the box under it found nothing, because the filter was
/// asking `object.name`, which the engine keeps in its one language.
/// Both names answer now: the one on the row, and the one the card is
/// known by everywhere outside this client.
#[test]
fn the_filter_answers_the_name_on_the_row_and_the_one_under_it() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Forest", 0),
                printed(5, 0, "Swamp", 0),
                printed(6, 0, "Llanowar Elves", 3),
            ],
        )
        .build();
    // The catalog, stood in for: this seat's printings are German.
    let german = |object: &baylee_view::PublicObject| match object.name.as_str() {
        "Forest" => Some("Wald".to_string()),
        "Swamp" => Some("Sumpf".to_string()),
        _ => None,
    };
    let names = Names { shown: &german };
    let found = |needle: &str| {
        let mut b = Browser::new();
        b.set_filter(needle);
        b.rows(&view, None, names)
            .into_iter()
            .map(|r| r.name)
            .collect::<Vec<_>>()
    };

    assert_eq!(found("wald"), ["Wald"], "the name the player is looking at");
    assert_eq!(found("forest"), ["Wald"], "the name they learned it under");
    assert_eq!(found("wal"), ["Wald"], "a prefix, which is how one types");
    assert!(found("sumpf") == ["Sumpf"] && found("mountain").is_empty());
    // A card the catalog has no German printing of keeps its own name,
    // and is still found by it — the fallback is a row, not a hole.
    assert_eq!(found("elves"), ["Llanowar Elves"]);
}

/// Both ends of the panel read the same alphabet.
///
/// The seam put German names on the rows and left them being compared as
/// bytes, which files every accented letter above `z`: a graveyard sorted
/// by name put *Ätherfluss* at the bottom, under *Zombie*. The filter had
/// the other half of it — nothing typed on a keyboard without an `ß`
/// could ever find a card printed with one.
#[test]
fn the_panel_alphabetises_and_searches_in_the_readers_own_letters() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Zombie", 2),
                printed(5, 0, "Aetherflux", 3),
                printed(6, 0, "Brainstorm", 1),
            ],
        )
        .build();
    // The same three cards, as this seat's printings name them.
    let german = |object: &baylee_view::PublicObject| {
        Some(match object.name.as_str() {
            "Aetherflux" => "Ätherfluss".to_string(),
            same => same.to_string(),
        })
    };
    let names = Names { shown: &german };

    let mut b = Browser::new();
    b.sort_by(SortKey::Name);
    let order: Vec<String> = b
        .rows(&view, None, names)
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert_eq!(
        order,
        ["Ätherfluss", "Brainstorm", "Zombie"],
        "the accent belongs at the front, with the A it is one of"
    );

    b.set_filter("atherfluss");
    assert_eq!(
        b.rows(&view, None, names).len(),
        1,
        "a keyboard with no umlaut still finds the card"
    );
    b.set_filter("ss");
    assert_eq!(
        b.rows(&view, None, names).len(),
        1,
        "and so does the ss in it"
    );
}

/// Two boxes ticked is one list, and no box ticked is every list.
///
/// The owner asked for the checkbox on 14.09.2026 — *"so das man sie
/// durch das checken quasi mergen kann"* — and the merge is the whole of
/// what has to be asserted: that the rows really do span both piles, that
/// they stay **grouped by zone** while they do (which is what
/// [`BrowseZone`]'s `Ord` is for, and the one property a set could have
/// thrown away), and that unticking the last box lands on everything
/// rather than on nothing. That last one is the empty set's meaning, and
/// a panel that could be emptied by a second click on one chip is the bug
/// this is written against.
#[test]
fn ticking_two_zones_merges_their_lists_and_unticking_the_last_shows_all() {
    let view = ViewBuilder::new(2)
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
        .build();
    let names = Names::projected();
    let mut b = Browser::new();
    b.open();
    assert!(b.shows_every_zone(), "a fresh panel is on Alle");
    assert_eq!(b.rows(&view, None, names).len(), 3, "all three piles");

    b.tick(BrowseZone::Graveyard(me()));
    assert_eq!(ticks(&b), vec![BrowseZone::Graveyard(me())]);
    assert_eq!(b.rows(&view, None, names).len(), 1, "one pile alone");

    b.tick(BrowseZone::Stack);
    assert_eq!(
        ticks(&b),
        vec![BrowseZone::Stack, BrowseZone::Graveyard(me())],
        "and they are held in tab order, not in the order they were ticked"
    );
    let zones: Vec<BrowseZone> = b
        .rows(&view, None, names)
        .iter()
        .map(|row| row.zone)
        .collect();
    assert_eq!(
        zones,
        vec![BrowseZone::Stack, BrowseZone::Graveyard(me())],
        "the merged list is still grouped by zone"
    );
    assert!(
        !b.shows(BrowseZone::Graveyard(PlayerId::new(1))),
        "a pile nobody ticked is not in the merge"
    );

    b.tick(BrowseZone::Stack);
    b.tick(BrowseZone::Graveyard(me()));
    assert!(b.shows_every_zone(), "the last tick off is Alle again");
    assert_eq!(b.rows(&view, None, names).len(), 3);
}

/// A ticked pile that empties takes its tick with it.
///
/// Nothing draws a chip for a zone with nothing in it ([`zones_of`]), so a
/// tick left behind on one is a state the player can see the effect of —
/// an empty list — and not the cause. `follow` is where it goes, because
/// that is the door a view comes in through.
#[test]
fn a_tick_does_not_outlive_the_pile_it_is_on() {
    let full = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .build();
    let mut b = Browser::new();
    b.open();
    b.tick(BrowseZone::Graveyard(me()));
    b.tick(BrowseZone::Stack);
    b.follow(&full, None);
    assert_eq!(ticks(&b).len(), 2, "both piles are still there");

    let emptied = ViewBuilder::new(2)
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .build();
    b.follow(&emptied, None);
    assert_eq!(
        ticks(&b),
        vec![BrowseZone::Stack],
        "the graveyard's tick went with the graveyard"
    );
}

#[test]
fn the_viewing_seats_own_piles_come_first() {
    let view = ViewBuilder::new(2)
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();
    assert_eq!(
        Browser::new().zones(&view),
        vec![
            BrowseZone::Looking,
            BrowseZone::Stack,
            BrowseZone::Graveyard(PlayerId::new(0)),
            BrowseZone::Graveyard(PlayerId::new(1)),
        ],
        "shown cards, then the stack, then mine, then theirs"
    );
}

#[test]
fn a_choice_for_another_seat_offers_nothing() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: PlayerId::new(1),
            options: vec![obj(7)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    assert!(!Browser::wanted(&view, &it));
    assert!(
        Browser::new()
            .rows(&view, Some(&it), Names::projected())
            .iter()
            .all(|r| !r.standing.selectable),
        "watching another seat choose is not choosing"
    );
}

/// The filter box is a field a player focuses, not a keyboard trap.
///
/// `set_filter` existed from the start and nothing ever called it: the
/// panel could sort and scroll, and the one thing the owner asked for by
/// name — "durchsuchbar" — had no way in. It is typed into now, and the
/// bargain is that it has to be *given* the keyboard: a box that took
/// every keystroke while the panel merely stood open would end playing
/// with the graveyard visible.
#[test]
fn the_filter_box_only_types_while_it_holds_the_keyboard() {
    let mut b = Browser::new();
    assert!(!b.is_typing(), "a fresh panel does not own the keyboard");

    b.start_typing();
    assert!(b.is_open(), "focusing the box opens the panel it lives in");
    assert!(b.is_typing());
    let focused = b.typing_epoch();

    for c in "Elv".chars() {
        b.push_filter(c);
    }
    b.push_filter('\n');
    assert_eq!(b.filter(), "Elv", "a control character reached the text");
    assert!(b.pop_filter());
    assert_eq!(b.filter(), "El");

    // The same rule on the path that needs it more. A keystroke is one
    // character a player meant; `set_filter` is a whole value arriving
    // from autofill or a paste, which is where a newline actually comes
    // from — and a filter holding one matches nothing at all.
    b.set_filter("Ll\tanowar\n");
    assert_eq!(
        b.filter(),
        "Llanowar",
        "a pasted value kept its control codes"
    );

    // Focusing again while already focused is not a new focus: a platform
    // input pointed at the box on every frame would fight the player for
    // the caret.
    b.start_typing();
    assert_eq!(b.typing_epoch(), focused);

    // Emptying the box, on the other hand, *is* one — the platform's own
    // field is still holding the old letters until something points it at
    // the new value.
    b.clear_filter();
    assert_eq!(b.filter(), "");
    assert!(b.typing_epoch() > focused, "the field was not re-seeded");
    b.set_filter("El");

    b.stop_typing();
    assert!(!b.is_typing());
    assert_eq!(
        b.filter(),
        "El",
        "letting go of the box threw the text away"
    );

    // And closing the panel lets go: the keyboard belongs to the game
    // again the moment the panel is not on screen.
    b.start_typing();
    b.close();
    assert!(!b.is_typing());
}

/// Typing narrows the rows, which is the whole point of the box.
#[test]
fn what_is_typed_is_what_is_listed() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(1, 0, "Elvish Mystic", 1),
                printed(2, 0, "Mountain", 2),
            ],
        )
        .build();
    let mut b = Browser::new();
    b.open();
    assert_eq!(b.rows(&view, None, Names::projected()).len(), 2);
    b.start_typing();
    for c in "mou".chars() {
        b.push_filter(c);
    }
    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1, "the filter did not reach the rows");
    assert_eq!(rows[0].name, "Mountain");
}

/// The search box is a field, not a string with letters pushed onto it.
///
/// The owner asked for it by naming the boxes that already work: *"die
/// Input felder überall, auch das Suchfeld im Zonen-Dialog soll
/// vollständig funktionieren wie ein normales Input Feld aus dem Web.
/// (Sowie die im Login Formullar, die funktionieren top.)"* Everything
/// below is something that box could not do — the caret could only ever
/// be at the end, because that is where `push`ing puts a character.
#[test]
fn the_search_box_has_a_caret_a_selection_and_the_keys_that_move_them() {
    use crate::textbuf::{Dir, Step};
    let mut b = Browser::new();
    b.start_typing();
    b.type_text("Llanowar Elves");

    // Home, then two words to the right, then type in the middle of it.
    b.move_filter_caret(Step::Line, Dir::Left, false);
    assert_eq!(b.filter_field().cursor(), 0);
    b.move_filter_caret(Step::Word, Dir::Right, false);
    assert_eq!(
        b.filter_field().cursor(),
        "Llanowar".len(),
        "a word is a word and not eight presses of the right arrow"
    );
    b.push_filter('!');
    assert_eq!(b.filter(), "Llanowar! Elves");

    // Shift extends a selection, and typing over it replaces it.
    b.move_filter_caret(Step::Line, Dir::Right, false);
    b.move_filter_caret(Step::Word, Dir::Left, true);
    assert_eq!(
        b.filter_field().selection().map(|s| &b.filter()[s]),
        Some("Elves"),
        "shift+⌥← selects the word behind the caret"
    );
    b.type_text("Mystic");
    assert_eq!(b.filter(), "Llanowar! Mystic");

    // Delete rubs out forwards, Backspace backwards, and either one takes
    // the selection whole when there is one.
    b.move_filter_caret(Step::Line, Dir::Left, false);
    b.delete_forward();
    assert_eq!(b.filter(), "lanowar! Mystic");
    b.select_all_filter();
    assert!(b.pop_filter(), "there was a selection to rub out");
    assert_eq!(b.filter(), "", "select-all and one press empties the box");
    assert!(!b.pop_filter(), "and there is nothing left to rub out");

    // A platform that owns its own typing hands the caret over with the
    // value, which is what an `<input>`'s arrow keys and its paste do.
    b.set_filter_state("Forest", 3, Some(6));
    assert_eq!(b.filter(), "Forest");
    assert_eq!(b.filter_field().cursor(), 3);
    assert_eq!(
        b.filter_field().selection().map(|s| &b.filter()[s]),
        Some("est")
    );
    // Unless a control character had to be dropped, which would move every
    // offset after it: then the caret goes to the end rather than
    // somewhere the platform did not mean.
    b.set_filter_state("For\nest", 3, Some(6));
    assert_eq!(b.filter(), "Forest");
    assert_eq!(b.filter_field().cursor(), 6);
    assert_eq!(b.filter_field().selection(), None);
}
