//! The framework's own checks, over screens recorded from a real client
//! (`tests/fixtures/`, written by `Client::keep`): they run in every gate,
//! with no window. Every layout check is shown to fire on a fault injected
//! into a recording, because a check that has only ever passed proves
//! nothing.
use crate::{
    layout::{Issue, Layout},
    screen::{Clip, ControlList, Screen, Target, card, in_hand, label, press, text},
};
use serde_json::{Value, json};

const FRONT_PHONE: &str = include_str!("../tests/fixtures/front-phone.json");
const FRONT_LAPTOP: &str = include_str!("../tests/fixtures/front-laptop.json");
const DUEL: &str = include_str!("../tests/fixtures/duel-mulligan.json");

fn screen(recorded: &str) -> Screen {
    Screen::from_recording(recorded).expect("a recording")
}

/// A recording with one edit made to its `/state`.
fn edited(recorded: &str, edit: impl FnOnce(&Screen, &mut Value)) -> Screen {
    let before = screen(recorded);
    let mut value: Value = serde_json::from_str(recorded).expect("JSON");
    edit(&before, &mut value["state"]);
    Screen::from_recording(&value.to_string()).expect("a recording")
}

/// The one pressable node a label names.
fn pressable(screen: &Screen, words: &str) -> usize {
    let found: Vec<usize> = (0..screen.nodes().len())
        .filter(|&i| screen.nodes()[i].pressable && screen.label_of(i) == words)
        .collect();
    assert_eq!(found.len(), 1, "{words:?}");
    found[0]
}

/// Moves a lobby node and everything under it (a node's index is its row in
/// `shell_nodes`, which comes first).
fn shift(screen: &Screen, state: &mut Value, node: usize, dx: f64, dy: f64) {
    for (i, _) in screen.subtree(node) {
        let row = &mut state["shell_nodes"][i];
        row["x"] = json!(row["x"].as_f64().unwrap() + dx);
        row["y"] = json!(row["y"].as_f64().unwrap() + dy);
    }
}

#[test]
fn the_tree_is_read_with_its_parents_its_roots_and_inherited_visibility() {
    for recorded in [FRONT_PHONE, FRONT_LAPTOP, DUEL] {
        let screen = screen(recorded);
        assert!(!screen.nodes().is_empty());
        for node in screen.nodes() {
            match node.parent {
                None => assert_eq!(node.depth, 0),
                Some(parent) => {
                    assert_eq!(screen.nodes()[parent].depth + 1, node.depth);
                    assert!(!screen.nodes()[parent].hidden || node.hidden);
                }
            }
        }
    }
    let duel = screen(DUEL);
    assert!(duel.in_duel());
    assert!(duel.nodes().iter().any(|n| n.root.as_deref() == Some("ui")));
    // The table's prompt is read off the HUD; the loading veil's words are
    // still in the tree, hidden, and are not shown.
    duel.expect_shows("Keep this hand?").unwrap();
    assert!(
        duel.nodes()
            .iter()
            .any(|n| n.text.as_deref() == Some("One table. A new story."))
    );
    duel.expect_not_shown("One table. A new story.").unwrap();
    assert!(!screen(FRONT_LAPTOP).in_duel());
}

#[test]
fn a_control_is_the_same_place_by_its_press_and_by_its_label() {
    // Two lists the harness writes apart: the controls with their `Press`
    // and the tree with its words. A label is joined to its control here.
    let front = screen(FRONT_LAPTOP);
    let by_press = front.locate(&press("Front(PlayOffline)")).unwrap();
    assert_eq!(front.locate(&label("Play offline")).unwrap(), by_press);
    let words = front.locate(&text("Play offline")).unwrap();
    assert!(by_press.contains(&words, 0.5), "{by_press} holds {words}");
}

#[test]
fn a_target_that_is_missing_or_ambiguous_is_refused_with_the_alternatives() {
    let front = screen(FRONT_PHONE);
    let both = front
        .locate(&press("Front(About(true))"))
        .unwrap_err()
        .to_string();
    assert!(
        both.contains("ambiguous") && both.contains("2 matches"),
        "{both}"
    );
    let missing = front
        .locate(&press("Front(Nowhere)"))
        .unwrap_err()
        .to_string();
    assert!(missing.contains("Front(PlayOffline)"), "{missing}");
    let unlabelled = front.locate(&label("Play online")).unwrap_err().to_string();
    assert!(unlabelled.contains("\"Play offline\""), "{unlabelled}");
    assert!(front.locate(&Target::At(3.0, 4.0)).is_ok());
}

#[test]
fn the_gateway_row_the_client_lists_by_itself_cannot_be_clicked() {
    let front = screen(FRONT_PHONE);
    let row = front.control("Front(SelectGateway(0))").unwrap().rect;
    let (x, y) = row.centre();
    let refused = front.refuse_a_real_gateway(x, y).unwrap_err().to_string();
    assert!(refused.contains("production"), "{refused}");
    let (x, y) = front.locate(&label("Play offline")).unwrap().centre();
    assert!(front.refuse_a_real_gateway(x, y).is_ok());
    // A saved row has a bin, and may be clicked.
    let saved = edited(FRONT_PHONE, |_, state| {
        state["lobby_controls"].as_array_mut().unwrap().push(
            json!({"press": "Front(ForgetGateway(0))", "at_x": 1, "at_y": 1, "w": 2, "h": 2}),
        );
    });
    let (x, y) = row.centre();
    assert!(saved.refuse_a_real_gateway(x, y).is_ok());
}

#[test]
fn at_the_table_a_card_a_button_and_the_cursor_are_read() {
    let duel = screen(DUEL);
    let hand: Vec<&str> = duel.cards_in("hand").map(|c| c.name.as_str()).collect();
    assert_eq!(hand.len(), 7, "{hand:?}");
    assert!(duel.locate(&in_hand(hand[0])).is_ok());
    assert!(duel.locate(&card("General Tazri")).is_ok());
    assert!(duel.locate(&in_hand("General Tazri")).is_err());
    let keep = duel.control("Keep").unwrap();
    assert_eq!(keep.list, ControlList::Table);
    assert_eq!(duel.locate(&press("Keep")).unwrap(), keep.rect);
    assert_eq!(duel.hovered(), None);
    let moved = edited(DUEL, |_, state| state["hovered"] = json!("ObjectId(79#0)"));
    assert_eq!(moved.hovered(), Some(79));
}

#[test]
fn the_recorded_screens_have_no_layout_issue() {
    for recorded in [FRONT_PHONE, FRONT_LAPTOP, DUEL] {
        let screen = screen(recorded);
        screen.expect_layout_ok().unwrap();
    }
}

#[test]
fn a_control_pushed_out_of_the_window_is_cut_off() {
    let moved = edited(FRONT_LAPTOP, |before, state| {
        let node = pressable(before, "Play offline");
        shift(before, state, node, 1000.0, 0.0);
    });
    let issues = moved.layout().issues(&moved);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i, Issue::CutOff { label, .. } if label == "Play offline")),
        "{issues:?}"
    );
}

#[test]
fn a_control_scrolled_down_its_list_is_not_misplaced_but_sideways_it_is_cut() {
    // The front door's panel scrolls down (`cy: scroll`) and not across.
    let down = edited(FRONT_LAPTOP, |before, state| {
        let node = pressable(before, "Play offline");
        shift(before, state, node, 0.0, 2000.0);
    });
    let node = pressable(&down, "Play offline");
    assert!(down.scrolled_away(node));
    assert_eq!(down.layout().issues(&down), []);
    let across = edited(FRONT_LAPTOP, |before, state| {
        let node = pressable(before, "Play offline");
        shift(before, state, node, 1000.0, 2000.0);
    });
    let node = pressable(&across, "Play offline");
    assert!(matches!(across.clipped_by(node), Some((_, Clip::Cut))));
}

#[test]
fn a_table_button_out_of_the_window_is_found() {
    let moved = edited(DUEL, |_, state| {
        for button in state["buttons"].as_array_mut().unwrap() {
            if button["label"] == "Keep" {
                button["at_x"] = json!(5000.0);
            }
        }
    });
    let issues = moved.layout().issues(&moved);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i, Issue::OutOfWindow { label, .. } if label == "Keep")),
        "{issues:?}"
    );
}

#[test]
fn a_text_wider_than_its_box_overflows() {
    for recorded in [FRONT_LAPTOP, DUEL] {
        let wide = edited(recorded, |before, state| {
            let at = before
                .nodes()
                .iter()
                .position(|n| !n.hidden && n.text_size.is_some_and(|(w, _)| w > 20.0))
                .unwrap();
            let (key, at) = if at < state["shell_nodes"].as_array().unwrap().len() {
                ("shell_nodes", at)
            } else {
                (
                    "ui_nodes",
                    at - state["shell_nodes"].as_array().unwrap().len(),
                )
            };
            let row = &mut state[key][at];
            row["tw"] = json!(row["w"].as_f64().unwrap() + 40.0);
        });
        let issues = wide.layout().issues(&wide);
        assert!(
            issues
                .iter()
                .any(|i| matches!(i, Issue::TextOverflow { .. })),
            "{issues:?}"
        );
        let allowed = Layout::desktop().may_overflow(match &issues[0] {
            Issue::TextOverflow { text, .. } => text,
            other => panic!("{other}"),
        });
        assert_eq!(allowed.issues(&wide), []);
    }
}

#[test]
fn two_controls_on_one_spot_overlap() {
    let stacked = edited(FRONT_LAPTOP, |before, state| {
        let about = pressable(before, "About");
        let offline = pressable(before, "Play offline");
        let (from, to) = (before.nodes()[about].rect, before.nodes()[offline].rect);
        shift(
            before,
            state,
            about,
            f64::from(to.x - from.x),
            f64::from(to.y - from.y),
        );
    });
    let issues = stacked.layout().issues(&stacked);
    assert!(
        issues.iter().any(|i| matches!(i, Issue::Overlap { .. })),
        "{issues:?}"
    );
    assert_eq!(
        stacked.layout().ignoring("About").issues(&stacked),
        [],
        "an ignored label is skipped by every check"
    );
}

#[test]
fn small_targets_are_a_touch_check_only() {
    let phone = screen(FRONT_PHONE);
    let touch = Layout::touch().issues(&phone);
    assert!(
        touch
            .iter()
            .any(|i| matches!(i, Issue::SmallTarget { label, .. } if label == "Play offline")),
        "{touch:?}"
    );
    assert_eq!(Layout::desktop().issues(&phone), []);
}

#[test]
fn a_recording_round_trips() {
    let front = screen(FRONT_PHONE);
    let again = Screen::from_recording(&front.recording().to_string()).unwrap();
    assert_eq!(again.nodes(), front.nodes());
    assert_eq!(again.controls(), front.controls());
    assert!(again.is_at(crate::Viewport::PHONE));
    assert!(!again.is_at(crate::Viewport::LAPTOP));
}
