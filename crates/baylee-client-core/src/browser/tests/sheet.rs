//! The sheet's own arithmetic: where the rectangle stands in the band, how far it can be dragged and stretched before it comes home, how many tiles fit across one row of the grid, and the one stored setting that says which shape a row is drawn in. All of it is pixels and stored taste; a view and a question appear only where they decide *whose* placement it is, because a sheet a question opened is centred on whatever window it meets and a sheet the player dragged stays where they put it. What the rows contain, which zones they come from and whether any of them may be picked is decided elsewhere - nothing here reads a `BrowseRow`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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
