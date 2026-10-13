//! The slash commands, row by row of §12.2 (every alias), and what a line that is no command becomes.

use baylee_protocol::social::Status;

use crate::social::commands::{
    COMMANDS, Command, CommandFault, GroupRef, Target, help_for, names_starting, parse, split_name,
};
use crate::social::words::ChatPhrase;

fn ok(line: &str) -> Command {
    parse(line).unwrap_or_else(|fault| panic!("{line:?} should parse, got {fault:?}"))
}

fn fault(line: &str) -> CommandFault {
    parse(line).expect_err(line)
}

fn say(to: Target, text: &str) -> Command {
    Command::Say {
        to,
        text: text.to_string(),
        emote: false,
    }
}

fn s(text: &str) -> String {
    text.to_string()
}

#[test]
fn table_aliases_say_to_the_table() {
    for alias in ["t", "s", "say", "table"] {
        assert_eq!(
            ok(&format!("/{alias} gg")),
            say(Target::Table, "gg"),
            "/{alias}"
        );
    }
}

#[test]
fn team_aliases_say_to_the_team() {
    for alias in ["team", "p", "party"] {
        assert_eq!(
            ok(&format!("/{alias} gg")),
            say(Target::Team, "gg"),
            "/{alias}"
        );
    }
}

#[test]
fn whisper_aliases_whisper_a_name() {
    for alias in ["w", "whisper", "tell", "msg"] {
        assert_eq!(
            ok(&format!("/{alias} Bob hello there")),
            Command::Whisper {
                who: s("Bob"),
                text: s("hello there")
            },
            "/{alias}"
        );
    }
}

#[test]
fn reply_aliases_reply() {
    for alias in ["r", "reply"] {
        assert_eq!(
            ok(&format!("/{alias} thanks")),
            Command::Reply(s("thanks")),
            "/{alias}"
        );
    }
    assert_eq!(ok("/r"), Command::Reply(String::new()));
}

#[test]
fn group_aliases_write_to_the_current_group() {
    for alias in ["g", "group"] {
        assert_eq!(
            ok(&format!("/{alias} hi all")),
            say(Target::Group(GroupRef::Current), "hi all"),
            "/{alias}"
        );
    }
}

#[test]
fn digits_one_to_nine_write_to_that_group() {
    for n in 1..=9_u8 {
        assert_eq!(
            ok(&format!("/{n} hi")),
            say(Target::Group(GroupRef::Number(n)), "hi"),
            "/{n}"
        );
    }
}

#[test]
fn a_group_number_is_read_without_counting_the_groups() {
    // `parse` is pure: whether there are three groups is the machine's to say.
    assert_eq!(ok("/1 hi"), say(Target::Group(GroupRef::Number(1)), "hi"));
    assert_eq!(ok("/9"), say(Target::Group(GroupRef::Number(9)), ""));
}

#[test]
fn numbers_outside_one_to_nine_are_unknown_commands() {
    for line in ["/0 hi", "/10 hi", "/11"] {
        assert!(matches!(fault(line), CommandFault::Unknown(_)), "{line}");
    }
}

#[test]
fn emote_aliases_make_an_emote_in_the_sticky_target() {
    for alias in ["me", "emote", "e"] {
        assert_eq!(
            ok(&format!("/{alias} shuffles")),
            Command::Say {
                to: Target::Sticky,
                text: s("shuffles"),
                emote: true
            },
            "/{alias}"
        );
    }
}

#[test]
fn name_commands_carry_their_name() {
    type Make = fn(String) -> Command;
    let rows: [(&[&str], Make); 8] = [
        (&["friend", "f", "add"], Command::Friend),
        (&["unfriend", "remove"], Command::Unfriend),
        (&["block"], Command::Block),
        (&["unblock"], Command::Unblock),
        (&["ignore", "mute"], Command::Ignore),
        (&["unignore", "unmute"], Command::Unignore),
        (&["invite", "inv"], Command::Invite),
        (&["watch", "spectate"], Command::Watch),
    ];
    for (aliases, make) in rows {
        for alias in aliases {
            assert_eq!(ok(&format!("/{alias} Bob")), make(s("Bob")), "/{alias} Bob");
            assert_eq!(
                ok(&format!("/{alias} Bob#0012")),
                make(s("Bob#0012")),
                "/{alias} Bob#0012"
            );
            assert_eq!(
                ok(&format!("/{alias} #0012")),
                make(s("#0012")),
                "/{alias} #0012"
            );
        }
    }
}

#[test]
fn a_name_command_drops_what_follows_the_name() {
    assert_eq!(
        ok("/block Bob#0012 for being rude"),
        Command::Block(s("Bob#0012"))
    );
    assert_eq!(ok("/friend Bob let's play"), Command::Friend(s("Bob")));
}

#[test]
fn mute_and_unmute_with_a_name_are_ignore_and_unignore() {
    assert_eq!(ok("/mute Bob"), Command::Ignore(s("Bob")));
    assert_eq!(ok("/unmute Bob"), Command::Unignore(s("Bob")));
}

#[test]
fn mute_and_unmute_without_a_name_mute_the_conversation() {
    for line in ["/mute", "/mute ", "/mute   "] {
        assert_eq!(ok(line), Command::Mute, "{line:?}");
    }
    for line in ["/unmute", "/unmute  "] {
        assert_eq!(ok(line), Command::Unmute, "{line:?}");
    }
}

#[test]
fn status_aliases_set_the_status() {
    let rows: [(&[&str], Status); 4] = [
        (&["online"], Status::Online),
        (&["away", "afk"], Status::Away),
        (&["dnd", "busy"], Status::Dnd),
        (&["invisible", "offline"], Status::Invisible),
    ];
    for (aliases, status) in rows {
        for alias in aliases {
            assert_eq!(
                ok(&format!("/{alias}")),
                Command::Status(status),
                "/{alias}"
            );
        }
    }
}

#[test]
fn friends_and_social_open_the_drawer() {
    for alias in ["friends", "social"] {
        assert_eq!(ok(&format!("/{alias}")), Command::Friends, "/{alias}");
    }
}

#[test]
fn who_leave_and_clear_take_no_arguments() {
    assert_eq!(ok("/who"), Command::Who);
    assert_eq!(ok("/leave"), Command::Leave);
    assert_eq!(ok("/clear"), Command::Clear);
}

#[test]
fn nick_with_a_nickname_sets_it() {
    assert_eq!(
        ok("/nick Bob Bobby"),
        Command::Nick {
            who: s("Bob"),
            name: Some(s("Bobby"))
        }
    );
    assert_eq!(
        ok("/nick Bob#0012 Big Bob"),
        Command::Nick {
            who: s("Bob#0012"),
            name: Some(s("Big Bob"))
        }
    );
    assert_eq!(
        ok("/nick Bob Smith#0012 Bobby"),
        Command::Nick {
            who: s("Bob Smith#0012"),
            name: Some(s("Bobby"))
        }
    );
}

#[test]
fn nick_without_a_nickname_clears_it() {
    assert_eq!(
        ok("/nick Bob"),
        Command::Nick {
            who: s("Bob"),
            name: None
        }
    );
    assert_eq!(
        ok("/nick Bob#0012   "),
        Command::Nick {
            who: s("Bob#0012"),
            name: None
        }
    );
}

#[test]
fn help_aliases_list_or_explain() {
    for alias in ["help", "?", "commands"] {
        assert_eq!(ok(&format!("/{alias}")), Command::Help(None), "/{alias}");
        assert_eq!(
            ok(&format!("/{alias} whisper")),
            Command::Help(Some(s("whisper"))),
            "/{alias} whisper"
        );
    }
}

#[test]
fn help_for_a_topic_drops_a_slash_and_lowercases() {
    assert_eq!(ok("/help /W"), Command::Help(Some(s("w"))));
    assert_eq!(ok("/? IGNORE"), Command::Help(Some(s("ignore"))));
}

#[test]
fn command_names_are_matched_without_regard_to_case() {
    assert_eq!(
        ok("/W Bob hi"),
        Command::Whisper {
            who: s("Bob"),
            text: s("hi")
        }
    );
    assert_eq!(ok("/TABLE gg"), say(Target::Table, "gg"));
    assert_eq!(ok("/Away"), Command::Status(Status::Away));
    assert_eq!(ok("/FRIEND Bob"), Command::Friend(s("Bob")));
    assert_eq!(ok("/MUTE"), Command::Mute);
}

#[test]
fn the_name_keeps_the_case_it_was_typed_in() {
    assert_eq!(ok("/block bOb"), Command::Block(s("bOb")));
}

#[test]
fn two_slashes_send_a_literal_slash_text() {
    assert_eq!(ok("//text"), say(Target::Sticky, "/text"));
    assert_eq!(ok("//w Bob hi"), say(Target::Sticky, "/w Bob hi"));
    assert_eq!(ok("///x"), say(Target::Sticky, "//x"));
}

#[test]
fn plain_text_says_to_the_sticky_target() {
    assert_eq!(ok("hello world"), say(Target::Sticky, "hello world"));
    assert_eq!(
        ok("a/b is not a command"),
        say(Target::Sticky, "a/b is not a command")
    );
    assert_eq!(ok("trailing   "), say(Target::Sticky, "trailing"));
}

#[test]
fn a_command_with_no_text_says_nothing_so_the_machine_can_switch_the_target() {
    assert_eq!(ok("/t"), say(Target::Table, ""));
    assert_eq!(ok("/team   "), say(Target::Team, ""));
    assert_eq!(ok("/g"), say(Target::Group(GroupRef::Current), ""));
}

#[test]
fn say_text_is_trimmed_of_the_gap_after_the_command() {
    assert_eq!(ok("/t    gg   wp  "), say(Target::Table, "gg   wp"));
}

#[test]
fn an_unknown_command_is_a_fault_with_the_name_as_typed() {
    assert_eq!(fault("/xyz hi"), CommandFault::Unknown(s("xyz")));
    assert_eq!(fault("/XYZ"), CommandFault::Unknown(s("XYZ")));
    assert_eq!(fault("/"), CommandFault::Unknown(String::new()));
    assert_eq!(
        fault("/whispers Bob hi"),
        CommandFault::Unknown(s("whispers"))
    );
}

#[test]
fn a_name_command_without_a_name_is_a_missing_name() {
    for alias in [
        "w", "whisper", "tell", "msg", "friend", "f", "add", "unfriend", "remove", "block",
        "unblock", "ignore", "unignore", "invite", "inv", "watch", "spectate", "nick",
    ] {
        assert_eq!(
            fault(&format!("/{alias}")),
            CommandFault::MissingName(s(alias)),
            "/{alias}"
        );
        assert_eq!(
            fault(&format!("/{alias}   ")),
            CommandFault::MissingName(s(alias)),
            "/{alias} with spaces"
        );
    }
}

#[test]
fn a_missing_name_names_the_alias_that_was_typed_in_lower_case() {
    assert_eq!(fault("/F"), CommandFault::MissingName(s("f")));
    assert_eq!(fault("/Whisper"), CommandFault::MissingName(s("whisper")));
}

#[test]
fn an_emote_without_text_is_a_missing_text() {
    for alias in ["me", "emote", "e"] {
        assert_eq!(
            fault(&format!("/{alias}")),
            CommandFault::MissingText(s(alias))
        );
        assert_eq!(
            fault(&format!("/{alias}   ")),
            CommandFault::MissingText(s(alias))
        );
    }
}

#[test]
fn a_whisper_with_a_name_and_no_text_switches_the_target() {
    assert_eq!(
        ok("/w Bob"),
        Command::Whisper {
            who: s("Bob"),
            text: String::new()
        }
    );
    assert_eq!(
        ok("/w Bob#0012  "),
        Command::Whisper {
            who: s("Bob#0012"),
            text: String::new()
        }
    );
}

#[test]
fn a_handle_with_a_space_before_its_tag_is_one_name() {
    assert_eq!(
        ok("/w Bob Smith#0012 hi"),
        Command::Whisper {
            who: s("Bob Smith#0012"),
            text: s("hi")
        }
    );
    assert_eq!(
        ok("/w Bob Smith#0012"),
        Command::Whisper {
            who: s("Bob Smith#0012"),
            text: String::new()
        }
    );
    assert_eq!(
        ok("/friend Bob Smith#0012"),
        Command::Friend(s("Bob Smith#0012"))
    );
}

#[test]
fn a_tag_alone_is_a_name() {
    assert_eq!(
        ok("/w #0012 hello"),
        Command::Whisper {
            who: s("#0012"),
            text: s("hello")
        }
    );
}

#[test]
fn a_hash_word_later_in_the_message_is_text_not_a_handle() {
    for (line, text) in [
        ("/w Bob see issue #1234", "see issue #1234"),
        ("/w Bob look at #beef", "look at #beef"),
        ("/w Bob #dead", "#dead"),
        ("/w Bob #0012 is open", "#0012 is open"),
    ] {
        assert_eq!(
            ok(line),
            Command::Whisper {
                who: s("Bob"),
                text: s(text)
            },
            "{line}"
        );
    }
    assert_eq!(
        split_name("Bob see issue #1234"),
        (s("Bob"), s("see issue #1234"))
    );
}

#[test]
fn a_hash_word_still_names_a_player_as_the_first_word() {
    assert_eq!(
        ok("/w #0012 see #1234"),
        Command::Whisper {
            who: s("#0012"),
            text: s("see #1234")
        }
    );
    assert_eq!(ok("/block #0012"), Command::Block(s("#0012")));
}

#[test]
fn a_spaced_handle_still_ends_at_its_tag_when_the_message_has_a_hash_word() {
    assert_eq!(
        ok("/w Bob Smith#0012 see #1234"),
        Command::Whisper {
            who: s("Bob Smith#0012"),
            text: s("see #1234")
        }
    );
}

#[test]
fn split_name_takes_the_first_word_unless_a_tagged_word_is_among_the_first_four() {
    let rows = [
        ("", "", ""),
        ("   ", "", ""),
        ("Bob", "Bob", ""),
        ("Bob hi", "Bob", "hi"),
        ("  Bob   hi there ", "Bob", "hi there"),
        ("Bob#0012 hi", "Bob#0012", "hi"),
        ("#0012 hi", "#0012", "hi"),
        ("Bob Smith#0012 hi", "Bob Smith#0012", "hi"),
        ("A B C#0012 hi", "A B C#0012", "hi"),
        ("A B C D#0012 hi", "A B C D#0012", "hi"),
        // Five words before a tag is past the reach: the first word is the name.
        ("A B C D E#0012 hi", "A", "B C D E#0012 hi"),
        // A tag must be a tag: 4–8 lowercase hex digits.
        ("Bob#zz hi", "Bob#zz", "hi"),
        ("Bob#12 hi", "Bob#12", "hi"),
        ("Bob#ABCD hi", "Bob#ABCD", "hi"),
    ];
    for (args, who, rest) in rows {
        assert_eq!(split_name(args), (s(who), s(rest)), "{args:?}");
    }
}

#[test]
fn the_fault_says_the_right_phrase() {
    assert_eq!(
        CommandFault::Unknown(s("xyz")).said(),
        (ChatPhrase::UnknownCommand, vec![s("xyz")])
    );
    assert_eq!(
        CommandFault::MissingName(s("w")).said(),
        (ChatPhrase::MissingName, vec![s("w")])
    );
    assert_eq!(
        CommandFault::MissingText(s("me")).said(),
        (ChatPhrase::MissingText, vec![s("me")])
    );
    assert_eq!(
        CommandFault::NotHere(Target::Team).said(),
        (ChatPhrase::NoTeam, Vec::new())
    );
    assert_eq!(
        CommandFault::NotHere(Target::Group(GroupRef::Number(3))).said(),
        (ChatPhrase::NoGroupNumber, vec![s("3")])
    );
    assert_eq!(
        CommandFault::NotHere(Target::Group(GroupRef::Current)).said(),
        (ChatPhrase::NoGroup, Vec::new())
    );
    assert_eq!(
        CommandFault::NotHere(Target::Whisper(s("Bob"))).said(),
        (ChatPhrase::NoReply, Vec::new())
    );
    assert_eq!(
        CommandFault::NotHere(Target::Table).said(),
        (ChatPhrase::NoTable, Vec::new())
    );
    assert_eq!(
        CommandFault::NotHere(Target::Sticky).said(),
        (ChatPhrase::NoTable, Vec::new())
    );
}

#[test]
fn help_for_finds_every_alias_of_every_row() {
    let rows: [(&[&str], ChatPhrase); 19] = [
        (&["t", "s", "say", "table"], ChatPhrase::HelpTable),
        (&["team", "p", "party"], ChatPhrase::HelpTeam),
        (&["w", "whisper", "tell", "msg"], ChatPhrase::HelpWhisper),
        (&["r", "reply"], ChatPhrase::HelpReply),
        (
            &["g", "group", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
            ChatPhrase::HelpGroup,
        ),
        (&["me", "emote", "e"], ChatPhrase::HelpEmote),
        (
            &["friend", "f", "add", "unfriend", "remove"],
            ChatPhrase::HelpFriend,
        ),
        (&["block", "unblock"], ChatPhrase::HelpBlock),
        (&["ignore", "unignore"], ChatPhrase::HelpIgnore),
        (&["invite", "inv"], ChatPhrase::HelpInvite),
        (&["watch", "spectate"], ChatPhrase::HelpWatch),
        (
            &[
                "online",
                "away",
                "afk",
                "dnd",
                "busy",
                "invisible",
                "offline",
            ],
            ChatPhrase::HelpStatus,
        ),
        (&["friends", "social"], ChatPhrase::HelpFriends),
        (&["who"], ChatPhrase::HelpWho),
        (&["nick"], ChatPhrase::HelpNick),
        (&["leave"], ChatPhrase::HelpLeave),
        (&["clear"], ChatPhrase::HelpClear),
        (&["mute", "unmute"], ChatPhrase::HelpMute),
        (&["help", "?", "commands"], ChatPhrase::HelpHelp),
    ];
    for (aliases, phrase) in rows {
        for alias in aliases {
            assert_eq!(help_for(alias), Some(phrase), "{alias}");
        }
    }
}

#[test]
fn help_for_ignores_a_leading_slash_and_case() {
    assert_eq!(help_for("/W"), Some(ChatPhrase::HelpWhisper));
    assert_eq!(help_for("Whisper"), Some(ChatPhrase::HelpWhisper));
    assert_eq!(help_for("/AFK"), Some(ChatPhrase::HelpStatus));
}

#[test]
fn help_for_an_unknown_command_is_none() {
    assert_eq!(help_for("xyz"), None);
    assert_eq!(help_for(""), None);
    assert_eq!(help_for("0"), None);
}

#[test]
fn every_listed_name_parses_as_a_command() {
    let mut seen = 0;
    for info in COMMANDS {
        for name in info.names {
            seen += 1;
            let line = format!("/{name} Bob");
            assert!(
                !matches!(parse(&line), Err(CommandFault::Unknown(_))),
                "{line} is listed and must be known"
            );
        }
    }
    assert!(seen >= 50, "the list has every alias, saw {seen}");
}

#[test]
fn no_name_is_listed_twice() {
    let mut names: Vec<&str> = COMMANDS
        .iter()
        .flat_map(|c| c.names.iter().copied())
        .collect();
    let all = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), all);
}

#[test]
fn names_starting_lists_names_and_aliases_in_help_order() {
    assert_eq!(names_starting("w"), ["w", "whisper", "watch", "who"]);
    assert_eq!(names_starting("t"), ["t", "table", "team", "tell"]);
    assert_eq!(
        names_starting("un"),
        ["unfriend", "unblock", "unignore", "unmute"]
    );
}

#[test]
fn names_starting_ignores_case_and_can_find_nothing() {
    assert_eq!(names_starting("WH"), ["whisper", "who"]);
    assert!(names_starting("zzz").is_empty());
}

#[test]
fn names_starting_with_nothing_lists_everything() {
    let all: usize = COMMANDS.iter().map(|c| c.names.len()).sum();
    assert_eq!(names_starting("").len(), all);
}
