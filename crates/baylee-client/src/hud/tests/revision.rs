//! The redraw gate has to read every field it carries.

/// A field of [`HudRevision`](crate::hud::HudRevision) that is compared
/// but never assigned redraws the tree on every frame; one that is
/// assigned but never compared is state that changes with nothing
/// happening on screen.
///
/// The second is not hypothetical. `choice` — which entry of the answer
/// chooser is picked — was drawn and never gated, and picking one never
/// leaves the client until Confirm, so nothing else in the struct moved
/// and the brass highlight stayed on whichever entry it had been on when
/// the tree was last built for some other reason. There was no way to
/// find that by reading the struct, because the struct looked complete.
///
/// Source-reading, for the reason the preview test above gives: the fact
/// is about a `Res<HudRevision>` inside a running renderer, and the
/// alternative is an `App` with a window in it.
#[test]
fn every_field_of_the_revision_is_both_compared_and_assigned() {
    let hud = include_str!("../../hud.rs");
    let overlay = include_str!("../overlay.rs");

    let body = hud
        .split_once("pub struct HudRevision {")
        .expect("the struct is still called that")
        .1;
    let body = body.split_once("\n}").expect("and still closes").0;
    let fields: Vec<&str> = body
        .lines()
        .filter_map(|line| {
            let name = line.strip_prefix("    ")?;
            let (name, _) = name.split_once(':')?;
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
                .then_some(name)
        })
        .collect();
    assert!(fields.len() > 15, "the fields did not parse: {fields:?}");

    let writes = overlay
        .find("revision.seq = seq;")
        .expect("the assignment block is still written out field by field");
    let (gate, assign) = overlay.split_at(writes);
    // From the anchor itself, not past it: `seq` is the field the gate
    // opens with.
    let opens = gate
        .rfind("if revision.seq == seq")
        .expect("and the gate above it");
    let gate: String = gate[opens..]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let assign: String = assign.chars().filter(|c| !c.is_whitespace()).collect();

    for field in fields {
        let needle = format!("revision.{field}");
        assert!(
            gate.contains(&needle),
            "`{field}` is remembered but never compared: it can change \
             with nothing on screen changing"
        );
        assert!(
            assign.contains(&needle),
            "`{field}` is compared but never written, so the tree rebuilds \
             every frame the moment it differs once"
        );
    }
}

/// The blind spot the test above has, one level down.
///
/// `browser` passed that test for as long as it existed, because the
/// whole field was compared and the whole field was assigned. It was a
/// four-tuple, and the browser has **seven** things a player can move:
/// the sort key and its direction were not in it, so clicking either of
/// the two buttons beside the filter box changed the order of a list that
/// was never redrawn. Both controls did nothing at all on screen. The
/// seventh is the view mode, added with the three shape buttons — and
/// added *here* first, precisely because three more buttons that redraw
/// nothing is the same defect a second time.
///
/// A struct with named fields is most of the fix — a literal that names
/// every field cannot forget one and still compile — so what is left to
/// guard is the escape hatch: `..Default::default()` would put the hole
/// straight back, with the compiler content.
///
/// Still seven, and not by having stood still. The sheet gaining a way out
/// on 19.09.2026 wanted an eighth — whether a *question* owns it, which
/// decides whether the two window buttons and the resize corner are drawn
/// at all — and `open` and that answer are one question wearing two bools:
/// `(open: false, for_choice: true)` is a shut sheet a question owns, which
/// no `Browser` is ever in. They are `hud::SheetOwner` now, so
/// the count below is a parse check and the honesty is in the type.
#[test]
fn the_browsers_gate_is_filled_field_by_field() {
    let hud = include_str!("../../hud.rs");
    // The dialog got a retained tree of its own, and the gate moved with
    // it: `sync_overlay` reads the browser nowhere at all any more.
    let built_in = include_str!("../tray.rs");

    let body = hud
        .split_once("struct BrowserGate {")
        .expect("the gate is still its own struct")
        .1
        .split_once("\n}")
        .expect("and still closes")
        .0;
    let fields: Vec<&str> = body
        .lines()
        .filter_map(|line| {
            let (name, _) = line.strip_prefix("    ")?.split_once(':')?;
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
                .then_some(name)
        })
        .collect();
    assert_eq!(fields.len(), 7, "the fields did not parse: {fields:?}");

    let built = built_in
        .split_once("let browser = super::BrowserGate {")
        .expect("still built where the gate is assembled")
        .1
        .split_once("\n    };")
        .expect("and still closes")
        .0;
    for field in fields {
        assert!(
            built.contains(&format!("{field}:")),
            "`{field}` is in the gate and is never read off the browser"
        );
    }
    assert!(
        !built.contains(".."),
        "a struct update fills the rest from `Default`, which is the hole \
         the named fields were supposed to close"
    );
}

/// The same blind spot again, one level further in.
///
/// Naming every field is not enough if a field is a *projection* that
/// throws state away. `filter` was the box's string, and the box draws
/// more than its string: `tray::filter_runs` asks `TextBuffer::segments`
/// for head, selection and tail and puts the caret bar between two of
/// them. So a caret that moved — every arrow, `Home`, `End`, ⌘A, and
/// every shift-held one of them — changed nothing the comparison could
/// see, and the box stood still until the *text* changed. Measured in the
/// running client on 14.09.2026: `abcdef`, then five `ArrowLeft`s, three
/// of them holding shift, moved the box by **zero** pixels — and the
/// `Backspace` after them deleted the **b**, which is the model saying
/// the caret had been standing at 2 the whole time.
///
/// Two halves, because the fix has two. The buffer has to notice a caret
/// in its own `PartialEq` — it does, which is what makes storing it
/// enough — and the gate has to store the buffer rather than ask it for
/// its string.
#[test]
fn the_gate_notices_a_caret_that_moved() {
    use baylee_client_core::textbuf::{Dir, Step, TextBuffer};

    let typed = TextBuffer::new("abcdef");
    let mut moved = typed.clone();
    moved.move_caret(Step::Char, Dir::Left, false);
    assert_ne!(
        typed, moved,
        "a buffer that compares equal with the caret somewhere else \
         cannot gate a redraw of the caret"
    );
    let mut held = typed.clone();
    held.move_caret(Step::Char, Dir::Left, true);
    assert_ne!(
        held, moved,
        "a selection and a bare caret at the same offset are two \
         different boxes on screen"
    );

    let built = include_str!("../tray.rs")
        .split_once("let browser = super::BrowserGate {")
        .expect("still built where the gate is assembled")
        .1
        .split_once("\n    };")
        .expect("and still closes")
        .0;
    let filter = built
        .lines()
        .find(|line| line.trim_start().starts_with("filter:"))
        .expect("the gate still has a filter");
    assert!(
        filter.contains("filter_field()"),
        "the gate reads `{}`, which is the text without the caret in it",
        filter.trim()
    );
}
