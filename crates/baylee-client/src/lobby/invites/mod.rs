//! Inviting friends (`docs/client.md` §"Inviting friends"; the owner,
//! 26.11.2026: Baylee Central's friend invites in the client): the sheet
//! the contacts drawer's "Invite a friend…" opens in the lobby, and an
//! invite a `baylee:` link brings ([`link`]).
//!
//! The sheet makes a single-use link at Baylee Central (`baylee_account`'s
//! invites), holding 1, 3, 7 or 14 days, with a name only the player sees,
//! a greeting for the invite's page and, where Central can mail, an address
//! Central mails it to. The new link stands once, with its QR code and a
//! copy button, over the form it came from (so nothing under it moves):
//! Central keeps only a digest of the code, and the client keeps the link
//! only while the sheet shows it, wiped when it goes. Under the form, the
//! account's invites (open, accepted, all), each open one with Withdraw,
//! which asks first.
//!
//! One request at a time on the account worker, never through the chat's
//! bridge: the list is read when the sheet opens and after every change,
//! and what was read before stands meanwhile, so a reopened sheet does not
//! flicker through a skeleton. The sheet keeps its frame, its fields and
//! their focus while parts of it are drawn again ([`draw`]).

use super::LobbyState;
use crate::DuelPhase;
use crate::shellkit::focus::{Activated, FocusSystems, ShellField, TabOrder};
use crate::ui_button::{Clicks, UiButton};
use baylee_account::social::{MAX_INVITE_LABEL, MAX_INVITE_NOTE};
use baylee_account::{CreatedInvite, Error, FriendInvites, NewInvite};
use baylee_client_core::social::ChatPhrase;
use baylee_client_core::social::invites::{self as model, Filter};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use std::sync::{Mutex, mpsc};
use zeroize::Zeroizing;

mod draw;
pub(crate) mod link;
#[cfg(test)]
mod tests;

/// The sheet's focus order.
pub(crate) const ORDER: TabOrder = TabOrder {
    name: "invites",
    stops: &[
        "close", "days", "label", "note", "email", "create", "copy", "another", "filter",
        "withdraw",
    ],
    modal: true,
};

/// The question before an invite is withdrawn.
pub(crate) const CONFIRM: TabOrder = TabOrder {
    name: "invites-withdraw",
    stops: &["cancel", "withdraw"],
    modal: true,
};

/// Opens the sheet: the contacts drawer's "Invite a friend…".
#[derive(Component, Clone, Copy, Debug)]
#[require(UiButton)]
pub(crate) struct OpenInvites;

/// What a control on the sheet does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(UiButton)]
pub(crate) enum Action {
    /// The `×`, `Esc`, or a press beside the sheet.
    Close,
    /// A press on the sheet itself, which closes nothing.
    Nothing,
    /// How many days a new link holds.
    Days(u8),
    /// Makes the link.
    Create,
    /// Puts the new link on the clipboard.
    Copy,
    /// Lays the new link aside and shows the form again.
    Another,
    /// Which invites the list shows.
    Filter(Filter),
    /// Asks before an open invite is withdrawn.
    Withdraw([u8; 16]),
    /// Withdraws the invite asked about.
    Confirm,
    /// Withdraws nothing.
    Cancel,
    /// Reads the list again after it failed.
    Retry,
}

/// Which of the form's fields a kit field is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    /// The name only the player sees.
    Label,
    /// The greeting on the invite's page.
    Note,
    /// Where Central mails the invite.
    Email,
}

impl Field {
    /// The most characters it takes: Central's own bounds.
    const fn most(self) -> usize {
        match self {
            Self::Label => MAX_INVITE_LABEL,
            Self::Note => MAX_INVITE_NOTE,
            Self::Email => baylee_account::MAX_INVITE_EMAIL,
        }
    }
}

/// A link just made: shown once, then wiped.
pub(crate) struct Made {
    /// The invite's page with its code.
    pub(crate) url: Zeroizing<String>,
    /// Its QR code, one texel a module, and its side in modules.
    pub(crate) code: Option<(Handle<Image>, u32)>,
    /// Until when it holds (Unix seconds).
    pub(crate) expires_at: i64,
    /// The address it was mailed to, and whether the mail went out.
    pub(crate) mail: Option<(String, bool)>,
    /// Which link this is, so the drawing knows a new one.
    pub(crate) serial: u64,
}

/// A request to Central.
enum Job {
    List,
    Create(NewInvite),
    Withdraw([u8; 16]),
}

/// Central's answer to a [`Job`].
enum Answer {
    Listed(Result<FriendInvites, Error>),
    Created(Result<CreatedInvite, Error>, Option<String>),
    Withdrawn(Result<(), Error>),
}

/// What the sheet stands on, and what it shows.
#[derive(Resource)]
pub(crate) struct Invites {
    open: bool,
    /// The sheet closed this frame: the key that closed it is still its.
    just_closed: bool,
    /// What Central listed last, and for which account.
    pub(crate) list: Option<FriendInvites>,
    owner: Option<String>,
    /// The list could not be read, and nothing was read before.
    pub(crate) failed: bool,
    pub(crate) filter: Filter,
    pub(crate) days: u8,
    pub(crate) made: Option<Made>,
    /// The open invite Withdraw asks about.
    pub(crate) asking: Option<[u8; 16]>,
    /// Why the last change was refused.
    pub(crate) said: Option<ChatPhrase>,
    /// How long "Copied" still stands, in seconds.
    pub(crate) copied: f32,
    /// The list is to be read again.
    stale: bool,
    work: Option<(Kind, Mutex<mpsc::Receiver<Answer>>)>,
    serial: u64,
}

/// What a request in flight does, for what the sheet holds meanwhile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    List,
    Create,
    Withdraw,
}

impl Default for Invites {
    fn default() -> Self {
        Self {
            open: false,
            just_closed: false,
            list: None,
            owner: None,
            failed: false,
            filter: Filter::default(),
            days: model::DEFAULT_DAYS,
            made: None,
            asking: None,
            said: None,
            copied: 0.0,
            stale: false,
            work: None,
            serial: 0,
        }
    }
}

impl Invites {
    /// Whether the sheet stands.
    pub(crate) fn open(&self) -> bool {
        self.open
    }

    /// Whether the sheet has the keys: while it stands, and the frame it
    /// closed in.
    pub(crate) fn holds_keys(&self) -> bool {
        self.open || self.just_closed
    }

    /// The request in flight.
    pub(crate) fn busy(&self) -> Option<Kind> {
        self.work.as_ref().map(|(kind, _)| *kind)
    }

    /// How many invites are open, and how many may be.
    pub(crate) fn slots(&self) -> Option<(usize, usize)> {
        self.list
            .as_ref()
            .map(|list| (model::open_count(&list.invites), list.active_limit as usize))
    }

    /// Every invite the account may have open is open.
    pub(crate) fn full(&self) -> bool {
        self.slots().is_some_and(|(open, most)| open >= most)
    }

    fn close(&mut self) {
        self.open = false;
        self.just_closed = true;
        // The link goes with the sheet, wiped.
        self.made = None;
        self.asking = None;
        self.said = None;
        self.copied = 0.0;
    }
}

/// Installs the sheet.
pub(super) fn install(app: &mut App) {
    app.init_resource::<Invites>()
        .add_systems(PreUpdate, |mut sheet: ResMut<Invites>| {
            if sheet.just_closed {
                sheet.just_closed = false;
            }
        })
        .add_systems(
            Update,
            (act.before(FocusSystems), cap_fields.after(FocusSystems)),
        )
        .add_systems(
            Update,
            (
                follow_the_session,
                ask,
                take_answers,
                draw::draw,
                draw::animate,
            )
                .chain()
                .after(act)
                .run_if(resource_exists::<crate::hud::UiFonts>),
        )
        .add_systems(OnEnter(DuelPhase::Opening), |mut sheet: ResMut<Invites>| {
            if sheet.open {
                sheet.close();
            }
        });
}

/// The sheet's presses, its keys and the drawer's button.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn act(
    mut opens: Clicks<OpenInvites>,
    mut clicks: Clicks<Action>,
    mut keyed: MessageReader<Activated>,
    actions: Query<&Action>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    focus: Option<Res<InputFocus>>,
    fields: Query<(&ShellField, &Field)>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    state: Option<Res<LobbyState>>,
    mut sheet: ResMut<Invites>,
) {
    if !opens.read().is_empty() && !sheet.open {
        sheet.open = true;
        sheet.stale = true;
        sheet.said = None;
    }
    let by_key: Vec<Action> = keyed
        .read()
        .filter(|a| a.by_key)
        .filter_map(|a| actions.get(a.entity).ok().copied())
        .collect();
    let mut pressed: Vec<Action> = clicks.read();
    pressed.extend(by_key);
    if !sheet.open {
        return;
    }
    let focused = focus.as_deref().and_then(InputFocus::get);
    let in_field = focused.and_then(|e| fields.get(e).ok());
    if let Some(codes) = codes.as_deref() {
        if codes.just_pressed(KeyCode::Escape) {
            // A field with text keeps its Esc: the kit clears it.
            if sheet.asking.is_some() {
                pressed.push(Action::Cancel);
            } else if in_field.is_none_or(|(field, _)| field.value.is_empty()) {
                pressed.push(Action::Close);
            }
        }
        if codes.just_pressed(KeyCode::Enter) && in_field.is_some() && sheet.asking.is_none() {
            pressed.push(Action::Create);
        }
    }
    for action in pressed {
        match action {
            Action::Close => sheet.close(),
            Action::Nothing => {}
            Action::Days(days) => {
                if sheet.days != days {
                    sheet.days = days;
                }
            }
            Action::Create => {
                if sheet.made.is_some() || sheet.busy().is_some() || sheet.full() {
                    continue;
                }
                let mut ask = NewInvite {
                    label: None,
                    note: None,
                    email: None,
                    lang: state
                        .as_deref()
                        .map_or("en", |s| s.lobby.lang().code())
                        .to_owned(),
                    days: sheet.days,
                };
                for (field, which) in &fields {
                    let value = Some(field.value.clone());
                    match which {
                        Field::Label => ask.label = value,
                        Field::Note => ask.note = value,
                        Field::Email => ask.email = value,
                    }
                }
                if ask
                    .email
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|a| !a.is_empty() && !baylee_account::plausible_email(a))
                {
                    sheet.said = Some(ChatPhrase::InviteRefused);
                    continue;
                }
                sheet.said = None;
                let email = ask
                    .email
                    .as_deref()
                    .map(str::trim)
                    .filter(|a| !a.is_empty())
                    .map(str::to_owned);
                start(
                    &mut sheet,
                    state.as_deref(),
                    Kind::Create,
                    Job::Create(ask),
                    email,
                );
            }
            Action::Copy => {
                if let (Some(made), Some(clipboard)) = (sheet.made.as_ref(), clipboard.as_mut()) {
                    let _ = clipboard.set_text(made.url.as_str().to_owned());
                    sheet.copied = COPIED_FOR;
                }
            }
            Action::Another => {
                sheet.made = None;
                sheet.copied = 0.0;
            }
            Action::Filter(filter) => {
                if sheet.filter != filter {
                    sheet.filter = filter;
                }
            }
            Action::Withdraw(id) => sheet.asking = Some(id),
            Action::Cancel => sheet.asking = None,
            Action::Confirm => {
                if let Some(id) = sheet.asking
                    && sheet.busy().is_none()
                {
                    start(
                        &mut sheet,
                        state.as_deref(),
                        Kind::Withdraw,
                        Job::Withdraw(id),
                        None,
                    );
                }
            }
            Action::Retry => {
                sheet.failed = false;
                sheet.stale = true;
            }
        }
    }
}

/// How long "Copied" stands under the link, in seconds.
pub(crate) const COPIED_FOR: f32 = 2.4;

/// Keeps each field within Central's bound for it.
fn cap_fields(mut fields: Query<(&mut ShellField, &Field), Changed<ShellField>>) {
    for (mut field, which) in &mut fields {
        let most = which.most();
        if field.value.chars().count() > most {
            let cut: String = field.value.chars().take(most).collect();
            field.value = cut;
        }
    }
}

/// The sheet belongs to the account it was opened for: signed out, or
/// another account, it closes and forgets what it read.
fn follow_the_session(state: Option<Res<LobbyState>>, mut sheet: ResMut<Invites>) {
    let owner = state.as_deref().and_then(account);
    if sheet.owner != owner {
        let sheet = &mut *sheet;
        sheet.owner = owner;
        sheet.list = None;
        sheet.failed = false;
        sheet.work = None;
        if sheet.open {
            sheet.close();
        }
    }
}

/// The signed-in Central account, by the handle its profile carries, or
/// `None` where nobody is signed in at Central.
fn account(state: &LobbyState) -> Option<String> {
    state.central_handle()?;
    state
        .central
        .as_ref()?
        .profile_handle
        .clone()
        .or_else(|| Some(String::new()))
}

/// Reads the list when it is stale and nothing else is under way. While the
/// sheet stands, a push from Central's social socket (someone took an
/// invite up, and the two are friends now) makes it stale: the list follows
/// without a press.
fn ask(state: Option<Res<LobbyState>>, mut sheet: ResMut<Invites>, mut pushed: Local<u64>) {
    let seq = state
        .as_deref()
        .and_then(LobbyState::central_handle)
        .map_or(0, |handle| handle.social_seq());
    if *pushed != seq {
        *pushed = seq;
        if sheet.open && !sheet.stale {
            sheet.stale = true;
        }
    }
    if !sheet.open || !sheet.stale || sheet.work.is_some() || sheet.failed {
        return;
    }
    sheet.stale = false;
    start(&mut sheet, state.as_deref(), Kind::List, Job::List, None);
}

/// Sends `job` on the account worker.
fn start(
    sheet: &mut Invites,
    state: Option<&LobbyState>,
    kind: Kind,
    job: Job,
    email: Option<String>,
) {
    let Some(handle) = state.and_then(LobbyState::central_handle) else {
        sheet.said = Some(ChatPhrase::InviteUnreachable);
        return;
    };
    let (send, receive) = mpsc::sync_channel(1);
    let sent = handle.submit(move |account| {
        let answer = match job {
            Job::List => Answer::Listed(account.friend_invites()),
            Job::Create(ask) => Answer::Created(account.create_friend_invite(&ask), email),
            Job::Withdraw(id) => Answer::Withdrawn(account.revoke_friend_invite(id)),
        };
        let _ = send.send(answer);
    });
    if sent {
        sheet.work = Some((kind, Mutex::new(receive)));
    } else {
        sheet.said = Some(ChatPhrase::InviteRefusedBusy);
    }
}

/// Takes in Central's answer, once it has come.
fn take_answers(
    state: Option<Res<LobbyState>>,
    images: Option<ResMut<Assets<Image>>>,
    mut fields: Query<&mut ShellField, With<Field>>,
    mut sheet: ResMut<Invites>,
) {
    let Some((kind, receive)) = sheet.work.as_ref() else {
        return;
    };
    let answer = match receive
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .try_recv()
    {
        Ok(answer) => answer,
        Err(mpsc::TryRecvError::Empty) => return,
        // The worker stopped (a sign-out): nothing will come.
        Err(mpsc::TryRecvError::Disconnected) => match kind {
            Kind::List => Answer::Listed(Err(Error::Unavailable)),
            Kind::Create => Answer::Created(Err(Error::Unavailable), None),
            Kind::Withdraw => Answer::Withdrawn(Err(Error::Unavailable)),
        },
    };
    let sheet = &mut *sheet;
    sheet.work = None;
    match answer {
        Answer::Listed(Ok(list)) => {
            sheet.list = Some(list);
            sheet.failed = false;
        }
        Answer::Listed(Err(_)) => sheet.failed = sheet.list.is_none(),
        Answer::Created(Ok(created), email) => {
            let lang = state
                .as_deref()
                .map_or(baylee_client_core::i18n::Lang::En, |s| s.lobby.lang());
            let url = Zeroizing::new(model::share_url(
                &super::source::central_website_url(),
                lang,
                &created.code,
            ));
            let code = images.and_then(|mut images| {
                let (image, side) = crate::shellkit::qr::image(&url)?;
                Some((images.add(image), side))
            });
            sheet.serial += 1;
            sheet.made = sheet.open.then(|| Made {
                url,
                code,
                expires_at: created.invite.expires_at,
                mail: email.map(|address| (address, created.mail_sent)),
                serial: sheet.serial,
            });
            sheet.copied = 0.0;
            sheet.stale = true;
            // The form the link came from is emptied for the next one.
            for mut field in &mut fields {
                if !field.value.is_empty() {
                    field.value.clear();
                }
            }
        }
        Answer::Created(Err(error), _) => sheet.said = Some(refusal(error)),
        Answer::Withdrawn(result) => {
            sheet.asking = None;
            if let Err(error) = result {
                sheet.said = Some(refusal(error));
            }
            sheet.stale = true;
        }
    }
}

/// Why Central said no, in the sheet's words.
pub(crate) fn refusal(error: Error) -> ChatPhrase {
    match error {
        Error::Busy | Error::Conflict => ChatPhrase::InviteRefusedBusy,
        Error::TermsRequired => ChatPhrase::InviteRefusedTerms,
        Error::Unavailable | Error::UpdateRequired => ChatPhrase::InviteUnreachable,
        Error::Rejected => ChatPhrase::InviteRefused,
    }
}
