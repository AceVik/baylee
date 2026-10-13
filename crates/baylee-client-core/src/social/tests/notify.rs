//! What a new line does, by kind, mute, ignore, do-not-disturb, focus, mention and the table's state (§10.6).

use crate::social::convs::ConvKind;
use crate::social::notify::{ChatCue, Notify, NotifyContext, on_line};
use crate::social::{ChatPrefs, ChatSounds};

const KINDS: [ConvKind; 4] = [
    ConvKind::Whisper,
    ConvKind::Group,
    ConvKind::Table,
    ConvKind::Team,
];

fn prefs() -> ChatPrefs {
    ChatPrefs {
        sounds: ChatSounds {
            whisper: true,
            mention: true,
            group: true,
            table: true,
            friend_online: false,
        },
        ..ChatPrefs::default()
    }
}

/// An unfocused window, away from the panel and the table.
fn ctx() -> NotifyContext {
    NotifyContext::default()
}

fn line(kind: ConvKind, ctx: NotifyContext) -> Notify {
    on_line(kind, false, false, &prefs(), ctx)
}

const NOTHING: Notify = Notify {
    badge: false,
    toast: false,
    sound: None,
    peek: false,
};

#[test]
fn a_whisper_gets_a_toast_a_sound_and_a_badge() {
    let n = line(ConvKind::Whisper, ctx());
    assert_eq!(
        n,
        Notify {
            badge: true,
            toast: true,
            sound: Some(ChatCue::Whisper),
            peek: false,
        }
    );
    // The same with the window focused but the panel on another tab.
    let n = line(
        ConvKind::Whisper,
        NotifyContext {
            focused: true,
            ..ctx()
        },
    );
    assert!(n.badge && n.toast && n.sound == Some(ChatCue::Whisper));
}

#[test]
fn a_mention_gets_a_toast_a_sound_and_a_badge_in_every_kind() {
    for kind in KINDS {
        let n = line(
            kind,
            NotifyContext {
                mention: true,
                focused: true,
                ..ctx()
            },
        );
        assert!(n.badge, "{kind:?}");
        assert!(n.toast, "{kind:?}");
        assert_eq!(n.sound, Some(ChatCue::Mention), "{kind:?}");
    }
}

#[test]
fn a_group_line_badges_and_sounds_only_while_the_window_has_no_focus() {
    let unfocused = line(ConvKind::Group, ctx());
    assert_eq!(
        unfocused,
        Notify {
            badge: true,
            sound: Some(ChatCue::GroupLine),
            ..NOTHING
        }
    );
    let focused = line(
        ConvKind::Group,
        NotifyContext {
            focused: true,
            ..ctx()
        },
    );
    assert_eq!(
        focused,
        Notify {
            badge: true,
            ..NOTHING
        }
    );
}

#[test]
fn a_table_or_team_line_badges_and_peeks_at_a_table_and_never_toasts() {
    for kind in [ConvKind::Table, ConvKind::Team] {
        let n = line(
            kind,
            NotifyContext {
                at_table: true,
                focused: true,
                ..ctx()
            },
        );
        assert!(n.badge && n.peek, "{kind:?}");
        assert!(!n.toast, "{kind:?}");
    }
}

#[test]
fn a_table_line_away_from_the_table_has_no_peek() {
    for kind in [ConvKind::Table, ConvKind::Team] {
        let n = line(kind, ctx());
        assert!(n.badge, "{kind:?}");
        assert!(!n.peek, "{kind:?}");
        assert!(!n.toast, "{kind:?}");
    }
}

#[test]
fn table_sound_follows_its_preference_and_is_off_by_default() {
    let on = prefs();
    let off = ChatPrefs {
        sounds: ChatSounds {
            table: false,
            ..on.sounds
        },
        ..on.clone()
    };
    for kind in [ConvKind::Table, ConvKind::Team] {
        let loud = on_line(kind, false, false, &on, ctx());
        assert_eq!(loud.sound, Some(ChatCue::GroupLine), "{kind:?}");
        let quiet = on_line(kind, false, false, &off, ctx());
        assert_eq!(quiet.sound, None, "{kind:?}");
        let default = on_line(kind, false, false, &ChatPrefs::default(), ctx());
        assert_eq!(default.sound, None, "default prefs, {kind:?}");
    }
}

#[test]
fn at_a_table_a_toast_becomes_a_peek_and_the_rest_stays() {
    let n = line(
        ConvKind::Whisper,
        NotifyContext {
            at_table: true,
            ..ctx()
        },
    );
    assert_eq!(
        n,
        Notify {
            badge: true,
            toast: false,
            sound: Some(ChatCue::Whisper),
            peek: true,
        }
    );
    let n = line(
        ConvKind::Group,
        NotifyContext {
            at_table: true,
            mention: true,
            ..ctx()
        },
    );
    assert!(n.badge && n.peek && !n.toast);
    assert_eq!(n.sound, Some(ChatCue::Mention));
}

#[test]
fn do_not_disturb_leaves_only_the_badge() {
    for kind in KINDS {
        for mention in [false, true] {
            let n = line(
                kind,
                NotifyContext {
                    dnd: true,
                    mention,
                    ..ctx()
                },
            );
            assert_eq!(
                n,
                Notify {
                    badge: true,
                    ..NOTHING
                },
                "{kind:?} mention={mention}"
            );
        }
    }
}

/// §10.6: "DND: badges only". `on_line` returns `peek: ctx.at_table` in its
/// `ctx.dnd` branch for every kind, so this fails today: a whisper or group
/// line at a table under DND still draws a peek line, while the same line in
/// a muted conversation does not.
#[test]
fn do_not_disturb_at_a_table_leaves_only_the_badge() {
    for kind in KINDS {
        let n = line(
            kind,
            NotifyContext {
                dnd: true,
                at_table: true,
                ..ctx()
            },
        );
        assert_eq!(
            n,
            Notify {
                badge: true,
                ..NOTHING
            },
            "{kind:?}"
        );
    }
}

#[test]
fn a_muted_conversation_does_nothing_but_peek_at_a_table() {
    for kind in KINDS {
        for mention in [false, true] {
            let n = on_line(
                kind,
                true,
                false,
                &prefs(),
                NotifyContext { mention, ..ctx() },
            );
            assert_eq!(n, NOTHING, "{kind:?} mention={mention}");
        }
    }
    for kind in [ConvKind::Table, ConvKind::Team] {
        let n = on_line(
            kind,
            true,
            false,
            &prefs(),
            NotifyContext {
                at_table: true,
                ..ctx()
            },
        );
        assert_eq!(
            n,
            Notify {
                peek: true,
                ..NOTHING
            },
            "{kind:?}"
        );
    }
    for kind in [ConvKind::Whisper, ConvKind::Group] {
        let n = on_line(
            kind,
            true,
            false,
            &prefs(),
            NotifyContext {
                at_table: true,
                ..ctx()
            },
        );
        assert_eq!(n, NOTHING, "a muted {kind:?} does not peek");
    }
}

#[test]
fn an_ignored_sender_does_nothing_at_all() {
    for kind in KINDS {
        let n = on_line(
            kind,
            false,
            true,
            &prefs(),
            NotifyContext {
                mention: true,
                at_table: true,
                ..ctx()
            },
        );
        assert_eq!(n, NOTHING, "{kind:?}");
    }
}

#[test]
fn reading_the_conversation_with_the_focus_does_nothing() {
    for kind in KINDS {
        let n = line(
            kind,
            NotifyContext {
                reading: true,
                focused: true,
                mention: true,
                at_table: true,
                ..ctx()
            },
        );
        assert_eq!(n, NOTHING, "{kind:?}");
    }
}

#[test]
fn reading_without_the_focus_still_notifies() {
    let n = line(
        ConvKind::Whisper,
        NotifyContext {
            reading: true,
            focused: false,
            ..ctx()
        },
    );
    assert!(n.badge && n.toast);
    assert_eq!(n.sound, Some(ChatCue::Whisper));
}

#[test]
fn a_question_at_the_table_silences_everything_but_a_whisper_or_a_mention() {
    let up = NotifyContext {
        question_up: true,
        at_table: true,
        ..ctx()
    };
    for kind in [ConvKind::Group, ConvKind::Table, ConvKind::Team] {
        let n = line(kind, up);
        assert!(n.badge, "{kind:?} still badges");
        assert_eq!(n.sound, None, "{kind:?}");
        assert!(!n.toast, "{kind:?}");
    }
    let whisper = line(ConvKind::Whisper, up);
    assert_eq!(whisper.sound, Some(ChatCue::Whisper));
    for kind in KINDS {
        let mention = line(
            kind,
            NotifyContext {
                mention: true,
                ..up
            },
        );
        assert_eq!(mention.sound, Some(ChatCue::Mention), "{kind:?}");
    }
}

#[test]
fn a_question_up_does_not_touch_the_toast_or_the_badge_of_a_whisper() {
    let n = line(
        ConvKind::Whisper,
        NotifyContext {
            question_up: true,
            ..ctx()
        },
    );
    assert!(n.badge && n.toast);
}

#[test]
fn each_sound_preference_switches_its_own_cue_off_and_leaves_the_rest() {
    let off = |f: fn(&mut ChatSounds)| {
        let mut p = prefs();
        f(&mut p.sounds);
        p
    };
    let n = on_line(
        ConvKind::Whisper,
        false,
        false,
        &off(|s| s.whisper = false),
        ctx(),
    );
    assert!(n.badge && n.toast, "a quiet whisper still toasts");
    assert_eq!(n.sound, None);

    let mention = NotifyContext {
        mention: true,
        ..ctx()
    };
    let n = on_line(
        ConvKind::Group,
        false,
        false,
        &off(|s| s.mention = false),
        mention,
    );
    assert!(n.badge && n.toast, "a quiet mention still toasts");
    assert_eq!(n.sound, None);
    // The whisper switch does not touch a mention in a whisper.
    let n = on_line(
        ConvKind::Whisper,
        false,
        false,
        &off(|s| s.whisper = false),
        mention,
    );
    assert_eq!(n.sound, Some(ChatCue::Mention));

    let n = on_line(
        ConvKind::Group,
        false,
        false,
        &off(|s| s.group = false),
        ctx(),
    );
    assert!(n.badge);
    assert_eq!(n.sound, None);
}

#[test]
fn the_matrix_obeys_the_rules_of_every_combination() {
    for kind in KINDS {
        for bits in 0_u16..256 {
            let bit = |i: u16| bits & (1 << i) != 0;
            let (muted, ignored) = (bit(0), bit(1));
            let c = NotifyContext {
                reading: bit(2),
                dnd: bit(3),
                focused: bit(4),
                question_up: bit(5),
                mention: bit(6),
                at_table: bit(7),
            };
            let n = on_line(kind, muted, ignored, &prefs(), c);
            let what = format!("{kind:?} muted={muted} ignored={ignored} {c:?}");
            check(kind, muted, ignored, c, n, &what);
        }
    }
}

fn check(kind: ConvKind, muted: bool, ignored: bool, c: NotifyContext, n: Notify, what: &str) {
    let table_kind = matches!(kind, ConvKind::Table | ConvKind::Team);
    if ignored || (c.reading && c.focused) {
        assert_eq!(n, NOTHING, "{what}");
        return;
    }
    if muted {
        assert_eq!(
            n,
            Notify {
                peek: c.at_table && table_kind,
                ..NOTHING
            },
            "{what}"
        );
        return;
    }
    assert!(n.badge, "{what}: an unmuted line badges");
    if c.dnd {
        // §10.6: "DND: badges only" (see `do_not_disturb_at_a_table_leaves_only_the_badge`).
        assert_eq!(
            n,
            Notify {
                badge: true,
                ..NOTHING
            },
            "{what}: dnd is badge only"
        );
        return;
    }
    assert_eq!(n.peek, c.at_table, "{what}");
    let urgent = c.mention || kind == ConvKind::Whisper;
    if n.toast {
        assert!(urgent && !c.at_table, "{what}: toast");
    }
    if urgent && !c.at_table {
        assert!(n.toast, "{what}: an urgent line away from a table toasts");
    }
    if c.question_up && !urgent {
        assert_eq!(n.sound, None, "{what}: a question silences it");
    }
    if urgent {
        assert!(n.sound.is_some(), "{what}: an urgent line sounds");
    }
    if let Some(cue) = n.sound {
        let expected = if c.mention {
            ChatCue::Mention
        } else if kind == ConvKind::Whisper {
            ChatCue::Whisper
        } else {
            ChatCue::GroupLine
        };
        assert_eq!(cue, expected, "{what}");
        if kind == ConvKind::Group && !c.mention {
            assert!(!c.focused, "{what}: group sound only unfocused");
        }
    }
}
