//! The invite sheet, headless: what opens and closes it, what it keeps,
//! and that the link it shows goes with it.

use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<Activated>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<InputFocus>();
    crate::ui_button::install(&mut app);
    app.init_resource::<Invites>()
        .add_systems(Update, act);
    app
}

fn made(serial: u64) -> Made {
    Made {
        url: Zeroizing::new("https://baylee.app/en/invite/Zm9vYmFyLWJheWxlZS0xMA".into()),
        code: None,
        expires_at: 0,
        mail: None,
        serial,
    }
}

#[test]
fn esc_closes_the_sheet_and_the_link_goes_with_it() {
    let mut app = app();
    {
        let mut sheet = app.world_mut().resource_mut::<Invites>();
        sheet.open = true;
        sheet.made = Some(made(1));
        sheet.asking = None;
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    let sheet = app.world().resource::<Invites>();
    assert!(!sheet.open());
    assert!(sheet.holds_keys(), "the Esc that closed it is still its");
    assert!(sheet.made.is_none(), "the link is wiped with the sheet");
}

#[test]
fn esc_over_the_question_answers_the_question_only() {
    let mut app = app();
    {
        let mut sheet = app.world_mut().resource_mut::<Invites>();
        sheet.open = true;
        sheet.asking = Some([3; 16]);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    let sheet = app.world().resource::<Invites>();
    assert!(sheet.open(), "the sheet stays");
    assert_eq!(sheet.asking, None, "the question goes");
}

#[test]
fn a_full_account_makes_no_link() {
    let mut sheet = Invites {
        list: Some(FriendInvites {
            invites: Vec::new(),
            active_limit: 0,
            mail_available: true,
            recruiter: None,
        }),
        ..Invites::default()
    };
    assert!(sheet.full());
    sheet.list = None;
    assert!(!sheet.full(), "unknown is not full");
}

#[test]
fn each_refusal_has_its_sentence() {
    assert_eq!(refusal(Error::Busy), ChatPhrase::InviteRefusedBusy);
    assert_eq!(refusal(Error::TermsRequired), ChatPhrase::InviteRefusedTerms);
    assert_eq!(refusal(Error::Unavailable), ChatPhrase::InviteUnreachable);
    assert_eq!(refusal(Error::Rejected), ChatPhrase::InviteRefused);
}

#[test]
fn a_field_keeps_within_central_s_bound() {
    let mut app = App::new();
    app.add_systems(Update, cap_fields);
    let note = app
        .world_mut()
        .spawn((ShellField::new(&"ä".repeat(300), ""), Field::Note))
        .id();
    app.update();
    let kept = &app.world().get::<ShellField>(note).unwrap().value;
    assert_eq!(kept.chars().count(), MAX_INVITE_NOTE);
}
