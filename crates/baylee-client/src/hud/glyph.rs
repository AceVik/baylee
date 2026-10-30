/// Hourglass from the shipped Font Awesome Free cmap (until end of turn).
pub const HOURGLASS: char = '\u{f254}';
/// Clock from the same cmap: a seat's decision clock beside its plate.
pub const CLOCK: char = '\u{f017}';
/// Heart (life total).
pub const HEART: char = '\u{f004}';
/// Hand (cards in hand).
pub const HAND: char = '\u{f256}';
/// Layer group (library).
pub const LIBRARY: char = '\u{f5fd}';
/// Skull (graveyard).
pub const SKULL: char = '\u{f54c}';
/// Ban (exile).
pub const EXILE: char = '\u{f05e}';
/// Battery half (charge counters on a permanent).
pub const CHARGE: char = '\u{f242}';
/// Bolt (energy counters).
///
/// Drawn by nothing since the commit that took the seat tab off the top
/// of the window: energy is one of the things the seat sheet will say
/// and neither the bar nor a player's button has room for.
// Used by `hud::tests`, which is where the icon face is read, so the
// expectation only holds in a build that is not the test one.
#[cfg_attr(not(test), expect(dead_code, reason = "the seat sheet says it next"))]
pub const ENERGY: char = '\u{f0e7}';
/// Caret down (speech-bubble tail).
pub const CARET_DOWN: char = '\u{f0d7}';
/// Expand (resize handle).
pub const EXPAND: char = '\u{f065}';
/// Crown (the command zone).
pub const COMMAND: char = '\u{f521}';
/// Crown again, as the monarch's (CR 724): on the monarch's plate and
/// chip, in the candle's gold (the owner, 08.10.2026). The same glyph as
/// [`COMMAND`], which is why commander damage left it.
pub const CROWN: char = '\u{f521}';
/// Chevron up: open the question's sheet from its pill.
pub const CHEVRON_UP: char = '\u{f077}';
/// Chevron down: fold the question's sheet to its pill.
pub const CHEVRON_DOWN: char = '\u{f078}';
/// Infinity: a hand no maximum size applies to, the mark alone.
pub const INFINITY: char = '\u{f534}';
/// Half a shield: the most damage one commander has dealt a seat
/// (CR 903.10a), on its plate and chip. It wore the crown until the
/// monarch needed it.
pub const COMMANDER_DAMAGE: char = '\u{f3ed}';
/// Robot (a chair the house plays, on a player's button, #264).
pub const HOUSE: char = '\u{f544}';
/// Person walking (a player who stepped away, whose chair is held).
pub const AWAY: char = '\u{f554}';
/// Times (close a panel). The text font has no U+2715, so the cross has
/// to come from here or it draws as a missing glyph.
pub const CLOSE: char = '\u{f00d}';
/// Check (a ticked box in the zone browser). Read out of the shipped
/// font's own cmap rather than looked up: a codepoint a search agrees
/// about is not the same claim as a glyph this file has.
pub const CHECK: char = '\u{f00c}';
/// A list of rows, each with a block at its left: the detailed view.
///
/// The three below are the zone browser's view buttons, and they are a
/// *set* — read out of the shipped font's cmap together and rendered
/// together, because what each one has to say it says by not looking like
/// the other two. This one is the only one of the three with a column
/// down its left side, which is the thumbnail.
pub const VIEW_ROWS: char = '\u{f00b}';
/// Three fat items with a mark beside each: the large view. Fewer things,
/// bigger, and no left column — which is exactly what that view drops.
pub const VIEW_BIG: char = '\u{f03a}';
/// Four large cells: the grid. Two by two rather than the denser
/// three-by-three beside it in the font, because the tiles are cards and
/// a grid of nine would read as a spreadsheet.
pub const VIEW_GRID: char = '\u{f009}';
/// Eye: show a masked field.
pub const EYE: char = '\u{f06e}';
/// Eye with a line through it: cover it again.
pub const EYE_SLASH: char = '\u{f070}';
/// Magnifier: this box is a search box.
///
/// Read out of the shipped font's own cmap and rendered, like the view
/// buttons above it — a codepoint a search agrees about is not the same
/// claim as a glyph this file has.
pub const MAGNIFIER: char = '\u{f002}';
/// Cogwheel: open the panel that builds what this box holds.
pub const GEAR: char = '\u{f013}';
/// Minus: take one away, and the `-` in front of a filter row.
pub const MINUS: char = '\u{f068}';
/// Plus: add one.
pub const PLUS: char = '\u{f067}';
/// A backspace arrow: take the last symbol off a mana cost.
pub const BACKSPACE: char = '\u{f55a}';
/// An `i` in a circle: the line under a panel that qualifies it.
pub const INFO: char = '\u{f05a}';
/// A window's bottom rule: put this sheet down into the tray.
///
/// Read out of the shipped font's own cmap and rasterised before it was
/// written here, like the view buttons and the magnifier above it. It
/// draws as a single horizontal bar and nothing else, which is why it is
/// this and not [`MINUS`]: a minus sign is an arithmetic operator the
/// drawer already uses for taking one off a number, and a control that
/// means two things in one window means neither.
pub const MINIMISE: char = '\u{f2d1}';
/// A filled window with a title bar: the sheet out to the whole band.
///
/// `window-maximize`, the neighbour of [`MINIMISE`] in the same font and
/// read out of the same cmap — glyph 833 against 834, which is what says
/// the pair was drawn as a pair. Not [`EXPAND`], which is the four arrows
/// the resize corner used to wear: those say "this can be made bigger",
/// where this one says "as big as it goes", and the corner is still there
/// saying the first thing.
pub const MAXIMISE: char = '\u{f2d0}';
/// Two overlapping frames: the sheet back to the size it was.
///
/// `window-restore`, glyph 835, the third of that run. It is a separate
/// mark rather than the same button meaning both, because a control
/// offering to maximise a sheet that already fills the band is the lie
/// this file already refuses to tell with a lit tab or an unlit Confirm.
pub const RESTORE: char = '\u{f2d2}';
/// A double-headed diagonal arrow: the sheet's resize corner.
///
/// It was [`EXPAND`] there, and that mark had to go the moment the sheet
/// grew a maximise button — four corner brackets say "out to the frame",
/// which is now the button's sentence and not the corner's. This says
/// "drag me and I get bigger or smaller", which is the only thing the
/// corner does now that its hidden second job is gone.
///
/// The font ships this diagonal and not the other one, so the arrows run
/// across the corner rather than along it. Two heads pointing apart is
/// what carries the meaning; which way the line leans is not read.
pub const RESIZE: char = '\u{f424}';
/// An archive box: the zone dialog, as a door on the tray.
///
/// Deliberately not [`LIBRARY`]'s layer-group, which is the obvious
/// picture of "a pile of cards" and is taken — a seat bar draws it for
/// the library, and two identical icons on one screen meaning two things
/// is worse than a less obvious one meaning its own. What this dialog
/// shows is the cards a game has put *away*: graveyards, exile, the
/// command zone, the stack. The box is the more exact of the two.
pub const ZONES: char = '\u{f187}';
/// The game menu's burger: three equal bars.
///
/// Read out of the shipped `fa-solid-900.ttf` and rastered before it was
/// written down, as every glyph here is. The near neighbour on this
/// screen is [`VIEW_BIG`], FA's "list", which is the same three bars with
/// bullets down their left and can stand in the zone browser's head while
/// this button shows; they are distinct at the sizes both are drawn at.
pub const BARS: char = '\u{f0c9}';
/// A scroll: the game log, as a door on the tray (#262).
///
/// Read out of the shipped `fa-solid-900.ttf` and rastered beside
/// [`ZONES`] and [`BARS`] before it was written down: at the tray's size
/// it is a rolled sheet, and neither a box nor a list. FA's "book" and
/// "file-lines" were the other two candidates; a book reads as rules, a
/// page with lines as a document to open.
pub const LOG: char = '\u{f70e}';
/// Comments, two overlapping speech bubbles: the chat's door (the beta.8
/// chat window design §4.1). Verified in the shipped
/// `fa-solid-900.ttf`; one bubble (`f4ad`, `f075`) reads as a single
/// message rather than a conversation.
pub const CHAT: char = '\u{f086}';
/// Lock: a conversation's encryption sheet.
pub const LOCK: char = '\u{f023}';
/// User group: the friends drawer.
pub const FRIENDS: char = '\u{f500}';
/// Paper plane: send.
pub const SEND: char = '\u{f1d8}';
/// Shield: the block list's blockers (the glyph commander damage wears).
pub const SHIELD: char = COMMANDER_DAMAGE;
/// Ellipsis: a row's menu.
pub const MORE: char = '\u{f141}';
