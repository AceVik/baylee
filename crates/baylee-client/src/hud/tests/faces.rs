//! The faces on disk, read as files rather than as handles.
//!
//! `AssetServer::load` is lazy and infallible: a name that matches nothing
//! hands back a handle that never resolves, so a renamed or forgotten `.ttf`
//! is an interface drawn in *nothing* and a test suite that says so
//! nowhere. These read the bytes.

/// Every face [`super::setup_fonts`] asks for, in its own spelling.
///
/// Typed out a second time on purpose. The point is to hold the loader's
/// strings against the directory, and a list built from the loader could
/// only ever agree with it.
const SHIPPED: [&str; 8] = [
    "AlegreyaSans-Regular.ttf",
    "AlegreyaSans-Medium.ttf",
    "AlegreyaSans-Bold.ttf",
    "AlegreyaSans-Italic.ttf",
    "AlegreyaSans-MediumItalic.ttf",
    "Faustina.ttf",
    "Faustina-Italic.ttf",
    "fa-solid-900.ttf",
];

fn path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/fonts")
        .join(file)
}

/// A TrueType table's offset and length, by tag.
///
/// Twenty lines of hand-rolled parser rather than a dependency, for the
/// same reason the sounds are computed: this reads two numbers out of one
/// table and a font crate in the tree would be a build cost paid on every
/// machine forever.
fn table(bytes: &[u8], tag: [u8; 4]) -> Option<(usize, usize)> {
    let count = u16::from_be_bytes([*bytes.get(4)?, *bytes.get(5)?]) as usize;
    (0..count).find_map(|i| {
        let at = 12 + 16 * i;
        (bytes.get(at..at + 4)? == tag).then(|| {
            let n = |o: usize| {
                u32::from_be_bytes([
                    bytes[at + o],
                    bytes[at + o + 1],
                    bytes[at + o + 2],
                    bytes[at + o + 3],
                ]) as usize
            };
            (n(8), n(12))
        })
    })
}

/// `OS/2`'s `usWeightClass`: 400 for a Regular, 700 for a Bold.
fn weight(bytes: &[u8]) -> Option<u16> {
    let (at, len) = table(bytes, *b"OS/2")?;
    (len >= 6).then(|| u16::from_be_bytes([bytes[at + 4], bytes[at + 5]]))
}

/// Every face the loader names is a file that is there.
#[test]
fn every_face_the_client_asks_for_is_in_the_tree() {
    for file in SHIPPED {
        let at = path(file);
        let bytes =
            std::fs::read(&at).unwrap_or_else(|_| panic!("{} is not shipped", at.display()));
        assert!(bytes.len() > 1024, "{file} is {} bytes", bytes.len());
        assert!(
            table(&bytes, *b"OS/2").is_some(),
            "{file} has no OS/2 table, so it is not a font this reads"
        );
    }
}

/// The bold cut is bold, and the other two weights of the family are not.
///
/// The whole of `tf_bold` rests on a *file*, because `TextFont::weight`
/// reaches only a variable font and these are static cuts. A file that
/// was quietly the Regular under a bold name would draw every control in
/// this client at the weight it had before, and nothing else in the suite
/// could tell.
#[test]
fn the_bold_cut_is_the_one_that_is_bold() {
    let of = |file: &str| weight(&std::fs::read(path(file)).expect("a face")).expect("a weight");
    assert_eq!(of("AlegreyaSans-Bold.ttf"), 700, "the bold cut is not bold");
    assert_eq!(of("AlegreyaSans-Medium.ttf"), 500);
    assert_eq!(of("AlegreyaSans-Regular.ttf"), 400);
}

/// Nothing under `assets/fonts` is unaccounted for.
///
/// `docs/legal.md` is the reason this is worth a test rather than a
/// glance: every file in there is a third party's, licensed under a
/// licence that has to be vendored beside it, and a face that arrived
/// without being named anywhere is exactly the kind of thing an audit is
/// supposed to find. The mana font is named by clause 2 and loaded
/// through `manaui`, so it is listed here as shipped-and-known.
#[test]
fn no_face_is_shipped_that_nothing_names() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .expect("the font directory")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().into_owned();
            std::path::Path::new(&name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ttf"))
                .then_some(name)
        })
        .collect();
    found.sort();
    let mut known: Vec<String> = SHIPPED
        .iter()
        .map(|s| (*s).to_string())
        .chain(std::iter::once("mana.ttf".to_string()))
        .collect();
    known.sort();
    assert_eq!(found, known, "a face nothing in the client names");
}

/// Which glyph a codepoint maps to in this face, or zero for none.
///
/// A `cmap` format-4 lookup, hand-rolled beside [`table`] and for the
/// same reason: it reads one number out of one table. Format 4 is the
/// BMP subtable every desktop face carries, and every character
/// `Chord::display` can produce is in the BMP.
fn glyph(bytes: &[u8], code: u32) -> u16 {
    let Some((cmap, _)) = table(bytes, *b"cmap") else {
        return 0;
    };
    let u16_at = |at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
    let subtables = u16_at(cmap + 2) as usize;
    // The Unicode BMP subtable: Windows/Unicode (3, 1), or Unicode (0, n).
    let sub = (0..subtables).find_map(|i| {
        let rec = cmap + 4 + 8 * i;
        let (platform, encoding) = (u16_at(rec), u16_at(rec + 2));
        let at = cmap
            + u32::from_be_bytes([
                bytes[rec + 4],
                bytes[rec + 5],
                bytes[rec + 6],
                bytes[rec + 7],
            ]) as usize;
        ((platform == 3 && encoding == 1) || platform == 0).then_some(at)
    });
    let Some(sub) = sub.filter(|at| u16_at(*at) == 4) else {
        return 0;
    };
    let Ok(code) = u16::try_from(code) else {
        return 0;
    };
    let segs = u16_at(sub + 6) as usize / 2;
    let ends = sub + 14;
    let starts = ends + 2 * segs + 2;
    let deltas = starts + 2 * segs;
    let ranges = deltas + 2 * segs;
    let Some(i) = (0..segs).find(|i| u16_at(ends + 2 * i) >= code) else {
        return 0;
    };
    let start = u16_at(starts + 2 * i);
    if start > code {
        return 0;
    }
    let delta = u16_at(deltas + 2 * i);
    let offset = u16_at(ranges + 2 * i) as usize;
    if offset == 0 {
        return code.wrapping_add(delta);
    }
    let at = ranges + 2 * i + offset + 2 * (code - start) as usize;
    match u16_at(at) {
        0 => 0,
        g => g.wrapping_add(delta),
    }
}

/// Every character a keyboard binding can be drawn with is one this face
/// actually has.
///
/// The defect that earned it: `Chord::display` spelled its modifiers with
/// the Mac marks `⌃⌥⇧⌘`, and **Alegreya Sans has none of the four**. They
/// drew an empty advance and nothing else, so the settings screen listed
/// `W` twice for `KeyW` and `Shift+W`, and the ledge's keycap promised
/// `Tab` for a turn that is skipped with `Shift+Tab`.
///
/// Read out of the shipped file rather than assumed, because a glyph is
/// the one thing about type that no amount of reading the source can
/// settle — and because the same trap is waiting for the next arrow,
/// mark or symbol somebody reaches for. The counter-test is what makes
/// the reader trustworthy: the four marks are checked to be *absent*, so
/// a `glyph` that answered yes to everything would fail here.
#[test]
fn every_chord_this_names_can_be_drawn() {
    use baylee_client_core::prefs::{Action, Keymap};
    let face = std::fs::read(path("AlegreyaSans-Bold.ttf")).expect("the bold cut");
    assert!(glyph(&face, 'W' as u32) > 0, "the reader found no W");

    let map = Keymap::standard();
    for action in Action::ALL {
        for chord in map.chords(action) {
            for ch in chord.display().chars() {
                assert!(
                    glyph(&face, ch as u32) > 0,
                    "{action:?} is drawn as {:?}, and this face has no \
                     {ch:?} — it renders as an empty advance, which is a \
                     key the player cannot read",
                    chord.display()
                );
            }
        }
    }

    // And the premise: those four marks really are missing, so the words
    // are a fix and not a preference.
    for mark in ['\u{2303}', '\u{2325}', '\u{21e7}', '\u{2318}'] {
        assert_eq!(
            glyph(&face, mark as u32),
            0,
            "this face has {mark:?} after all — the modifiers could be \
             marks again, and this test is the place to decide that"
        );
    }
    // The arrows, which are kept precisely because they are there.
    for arrow in ['\u{2190}', '\u{2191}', '\u{2192}', '\u{2193}'] {
        assert!(glyph(&face, arrow as u32) > 0, "no {arrow:?}");
    }
}

/// Every mark `hud::glyph` names is one the icon face actually has.
///
/// The same trap as the chords above, in the font it is easiest to fall
/// into: a Font Awesome codepoint is a four-digit number nobody can read,
/// a search will agree about one cheerfully, and a wrong one draws a
/// blank advance that looks exactly like a control the layout forgot. The
/// close button shipped as a thin bar once for a version of this.
///
/// Typed out a second time, like [`SHIPPED`]: a list built from the
/// module could only ever agree with the module.
#[test]
fn every_mark_the_overlay_names_is_in_the_icon_face() {
    use crate::hud::glyph;
    let face = std::fs::read(path("fa-solid-900.ttf")).expect("the icon face");
    let named = [
        ("HEART", glyph::HEART),
        ("HAND", glyph::HAND),
        ("LIBRARY", glyph::LIBRARY),
        ("SKULL", glyph::SKULL),
        ("EXILE", glyph::EXILE),
        ("POISON", glyph::POISON),
        ("ENERGY", glyph::ENERGY),
        ("CARET_DOWN", glyph::CARET_DOWN),
        ("EXPAND", glyph::EXPAND),
        ("COMMAND", glyph::COMMAND),
        ("CLOSE", glyph::CLOSE),
        ("CHECK", glyph::CHECK),
        ("VIEW_ROWS", glyph::VIEW_ROWS),
        ("VIEW_BIG", glyph::VIEW_BIG),
        ("VIEW_GRID", glyph::VIEW_GRID),
        ("EYE", glyph::EYE),
        ("EYE_SLASH", glyph::EYE_SLASH),
    ];
    for (name, mark) in named {
        assert!(
            glyph(&face, mark as u32) > 0,
            "glyph::{name} is U+{:04X}, which this face does not have — it draws as an \
             empty advance, which reads as a control that is simply missing",
            mark as u32
        );
    }

    // The three view marks have to differ from one another as well as
    // exist: they are one control with three answers, and two segments
    // drawn alike is a button that cannot say what it does.
    let views = [glyph::VIEW_ROWS, glyph::VIEW_BIG, glyph::VIEW_GRID];
    let shapes: std::collections::BTreeSet<u16> =
        views.iter().map(|m| glyph(&face, *m as u32)).collect();
    assert_eq!(shapes.len(), 3, "two of the three view marks are one glyph");

    // And the premise: this reader can say no. U+F999 sits *inside* the
    // private-use block the marks above come from and this face does not
    // fill it, which is the counter-test worth having — a reader that
    // answered "yes" to the whole block would pass every assertion above
    // while checking nothing. (U+F8FF is not the one to use: the face
    // does fill that slot, which is how this test first failed.)
    assert_eq!(
        glyph(&face, 0xf999),
        0,
        "the reader answers yes to a codepoint the face has no glyph for"
    );
}
