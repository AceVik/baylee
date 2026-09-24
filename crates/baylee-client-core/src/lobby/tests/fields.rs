//! One caret and the boxes it moves between: placing it counts even when it does not move, each form shape has its own Tab ring, tabbing selects what is there where clicking only places, and the platform may hand back a caret that moved under text that did not. Typing, backspace and delete land at the caret rather than at the end, and a revealed password covers itself again the moment the caret leaves it. `FieldKind` belongs here as well, because what a box asks a password manager for is a property of the box and not of the form. Nothing here submits anything — what a filled-in form then does is `sign_in`, and the room password box is spent in `rooms`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn placing_the_caret_is_visible_even_when_it_does_not_move() {
    let mut lobby = Lobby::new();
    let start = lobby.focus_epoch();
    lobby.focus_on(Field::Email);
    assert!(
        lobby.focus_epoch() > start,
        "tapping the field you are already in still has to raise a keyboard"
    );
    let again = lobby.focus_epoch();
    lobby.cycle_focus(Tab::Next);
    assert!(lobby.focus_epoch() > again);
    let refused = lobby.focus_epoch();
    lobby.focus_on(Field::DisplayName);
    assert_eq!(
        lobby.focus_epoch(),
        refused,
        "a field that is not on screen is not focused, so nothing happens"
    );
}

#[test]
fn a_field_can_be_replaced_wholesale() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Email, "pasted@example.com");
    assert_eq!(lobby.field(Field::Email), "pasted@example.com");
    lobby.set_field(Field::Email, "");
    assert_eq!(lobby.field(Field::Email), "", "clearing works too");
}

#[test]
fn a_password_can_be_shown_and_covered_again() {
    let mut lobby = Lobby::new();
    assert!(!lobby.showing(Field::Password));
    lobby.toggle_reveal(Field::Password);
    assert!(lobby.showing(Field::Password));
    assert_eq!(
        lobby.focus(),
        Field::Password,
        "pressing the eye beside a box is a way of saying that box"
    );
    lobby.toggle_reveal(Field::Password);
    assert!(!lobby.showing(Field::Password));
}

#[test]
fn the_eye_leaves_a_caret_that_is_already_in_the_box_alone() {
    let mut lobby = Lobby::new();
    lobby.focus_on(Field::Password);
    lobby.set_field_at(Field::Password, "hunter2", 3, None);
    let epoch = lobby.focus_epoch();
    lobby.toggle_reveal(Field::Password);
    assert!(lobby.showing(Field::Password));
    assert_eq!(
        lobby.focus_epoch(),
        epoch,
        "the caret was already there, so nothing re-opens a browser's own input"
    );
    assert_eq!(
        lobby.buffer(Field::Password).cursor(),
        3,
        "and the caret stays where the player was typing"
    );
}

#[test]
fn a_shown_password_is_covered_again_by_leaving_it() {
    let mut lobby = Lobby::new();
    lobby.toggle_reveal(Field::Password);
    lobby.focus_on(Field::Email);
    assert!(
        !lobby.showing(Field::Password),
        "the caret left, so the secret is a secret again"
    );
    lobby.toggle_reveal(Field::Password);
    lobby.cycle_focus(Tab::Next);
    assert!(!lobby.showing(Field::Password), "and Tab is leaving too");
    lobby.toggle_reveal(Field::RoomPassword);
    assert!(
        !lobby.showing(Field::Password),
        "one at a time: a room's password is not the account's"
    );
}

#[test]
fn a_field_says_what_kind_of_keyboard_it_wants() {
    let lobby = Lobby::new();
    assert_eq!(lobby.field_kind(Field::Email), FieldKind::Email);
    assert_eq!(lobby.field_kind(Field::DisplayName), FieldKind::Name);
    assert_eq!(lobby.field_kind(Field::Password), FieldKind::Password);
    assert_eq!(
        lobby.field_kind(Field::RoomPassword),
        FieldKind::Secret,
        "a room's password is not the account's and must not autofill as it"
    );
}

#[test]
fn the_password_box_asks_for_a_new_password_on_the_sign_up_form() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.field_kind(Field::Password), FieldKind::Password);
    let before = lobby.focus_epoch();
    lobby.toggle_registering();
    assert_eq!(
        lobby.field_kind(Field::Password),
        FieldKind::NewPassword,
        "the same box, and the opposite request to a password manager"
    );
    lobby.focus_on(Field::Password);
    let placed = lobby.focus_epoch();
    lobby.toggle_registering();
    assert!(
        lobby.focus_epoch() > placed,
        "flipping the form under the caret has to re-point the platform's input"
    );
    assert!(before < placed, "the premise: placing the caret counts");
}

#[test]
fn shift_tab_walks_the_sign_up_form_backwards() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    assert_eq!(lobby.focus(), Field::Email);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(
        lobby.focus(),
        Field::PasswordAgain,
        "back from the first is last"
    );
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Password);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::DisplayName);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Email);
}

#[test]
fn tab_walks_the_sign_up_form_in_the_order_it_is_drawn() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    let mut walked = vec![lobby.focus()];
    for _ in 0..4 {
        lobby.cycle_focus(Tab::Next);
        walked.push(lobby.focus());
    }
    assert_eq!(
        walked,
        [
            Field::Email,
            Field::DisplayName,
            Field::Password,
            Field::PasswordAgain,
            Field::Email
        ]
    );
    assert!(lobby.typing_here());
    lobby.focus_on(Field::PasswordAgain);
    assert!(lobby.typing_here());
    assert_eq!(
        lobby.field_kind(Field::PasswordAgain),
        FieldKind::NewPassword,
        "a manager offers the password it just made, not the saved one"
    );
    lobby.toggle_registering();
    assert_eq!(
        lobby.focus(),
        Field::Password,
        "leaving sign-up takes the caret out of the box that goes away"
    );
}

/// A ring of two reverses to itself, so the log-in form answers Tab and
/// ⇧Tab alike — which is what a browser does with two fields as well.
#[test]
fn shift_tab_on_the_log_in_form_is_tab() {
    let mut lobby = Lobby::new();
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Password);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Email);
}

/// Tabbing into a field selects it, so the next character replaces what
/// is there. An append-only field could not do that, which is why an
/// address typed one letter wrong had to be deleted back to the mistake.
#[test]
fn tabbing_into_a_field_selects_what_is_in_it() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Password, "wrong");
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Password);
    assert_eq!(lobby.buffer(Field::Password).selection(), Some(0..5));
    lobby.type_char('r');
    assert_eq!(lobby.field(Field::Password), "r");
}

/// Clicking into a field is not tabbing into it: the caret is placed,
/// nothing is selected, and typing goes on from there.
#[test]
fn clicking_into_a_field_does_not_select_it() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Password, "half");
    lobby.focus_on(Field::Password);
    assert_eq!(lobby.buffer(Field::Password).selection(), None);
    lobby.type_char('!');
    assert_eq!(lobby.field(Field::Password), "half!");
}

/// What a browser's `<input>` reports after the player moved its caret:
/// the text is unchanged and only the caret moved, which
/// [`Lobby::set_field`] discards as "no change".
#[test]
fn the_platform_may_move_the_caret_without_changing_the_text() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Email, "mail@example.com");
    lobby.set_field_at(Field::Email, "mail@example.com", 4, Some(0));
    assert_eq!(lobby.buffer(Field::Email).cursor(), 4);
    assert_eq!(lobby.buffer(Field::Email).selection(), Some(0..4));
    lobby.type_char('n');
    assert_eq!(lobby.field(Field::Email), "n@example.com");
}

/// The caret is a caret and not an append cursor: a correction made in
/// the middle of an address lands in the middle of it.
#[test]
fn typing_lands_at_the_caret_and_backspace_takes_what_is_before_it() {
    let mut lobby = Lobby::new();
    lobby.set_field_at(Field::Email, "mailexample.com", 4, None);
    lobby.type_char('@');
    assert_eq!(lobby.field(Field::Email), "mail@example.com");
    lobby.backspace();
    assert_eq!(lobby.field(Field::Email), "mailexample.com");
    lobby.delete_forward();
    assert_eq!(lobby.field(Field::Email), "mailxample.com");
    lobby.move_caret(Reach::Line, Dir::Left, false);
    lobby.type_char('e');
    assert_eq!(
        lobby.field(Field::Email),
        "emailxample.com",
        "Home, then type"
    );
}

#[test]
fn tab_skips_the_display_name_when_logging_in() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.focus(), Field::Email);
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Password);
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Email);
    lobby.toggle_registering();
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::DisplayName);
}

#[test]
fn leaving_the_sign_up_form_moves_the_caret_off_a_hidden_field() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.focus_on(Field::DisplayName);
    lobby.toggle_registering();
    assert_eq!(lobby.focus(), Field::Password);
}

#[test]
fn typing_lands_in_the_focused_field_and_control_keys_do_not() {
    let mut lobby = Lobby::new();
    lobby.type_char('h');
    lobby.type_char('\n');
    lobby.type_char('i');
    assert_eq!(lobby.field(Field::Email), "hi");
    lobby.backspace();
    assert_eq!(lobby.field(Field::Email), "h");
    lobby.backspace();
    lobby.backspace();
    assert_eq!(lobby.field(Field::Email), "", "an empty field survives");
}

#[test]
fn the_front_door_types_only_into_the_form_it_shows() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Email, "mail@example.com");

    lobby.set_gateway_ready(false);
    assert!(!lobby.gateway_chosen());
    assert_eq!(lobby.focus(), Field::Gateway);
    assert!(lobby.typing_here());
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Gateway, "a ring of one is that field");
    lobby.focus_on(Field::Password);
    assert!(
        !lobby.typing_here(),
        "the account form is not on screen, so a password typed now goes nowhere"
    );

    lobby.set_gateway_ready(true);
    assert_eq!(
        lobby.focus(),
        Field::Password,
        "the address is filled in, so the password is what is missing"
    );
    assert!(lobby.typing_here());
    lobby.focus_on(Field::Gateway);
    assert!(
        !lobby.typing_here(),
        "and the gateway form is not on screen"
    );
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Email);

    lobby.set_field(Field::Email, " ");
    lobby.set_gateway_ready(false);
    lobby.set_gateway_ready(true);
    assert_eq!(
        lobby.focus(),
        Field::Email,
        "nothing filled in: from the top"
    );
}
