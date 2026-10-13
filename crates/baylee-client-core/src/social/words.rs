//! What the chat says, in every language (§10.1): its own list, so the
//! chat's words don't grow [`crate::i18n::Phrase`]'s.

use crate::i18n::{Lang, messages};

messages! {
    /// One thing the chat says.
    ChatPhrase;
    /// A line that failed its signature, wrap or body check.
    MessageDropped { en: "A message failed its check and was dropped", de: "Eine Nachricht hat die Prüfung nicht bestanden und wurde verworfen" },
    /// A line from a device this client couldn't find.
    UnknownDevice { en: "A message from {0} came from a device that couldn't be checked and was dropped", de: "Eine Nachricht von {0} kam von einem Gerät, das nicht geprüft werden konnte, und wurde verworfen" },
    /// A line addressed somewhere it doesn't belong.
    MisaddressedDropped { en: "A message that didn't belong in this conversation was dropped", de: "Eine Nachricht, die nicht in dieses Gespräch gehörte, wurde verworfen" },
    /// A contact added a device.
    DeviceAdded { en: "{0} added a device ({1} · {2})", de: "{0} hat ein Gerät hinzugefügt ({1} · {2})" },
    /// A contact removed a device.
    DeviceRemoved { en: "{0} removed a device", de: "{0} hat ein Gerät entfernt" },
    /// A verified contact's devices changed.
    VerifyAgain { en: "{0}'s devices changed · Verify again", de: "Die Geräte von {0} haben sich geändert · Erneut bestätigen" },
    /// This account gained a device.
    OwnDeviceAdded { en: "A new device was added to your account: {0} · {1}. Not you? Remove it and change your password.", de: "Deinem Konto wurde ein neues Gerät hinzugefügt: {0} · {1}. Warst du das nicht? Entferne es und ändere dein Passwort." },
    /// The gateway listed someone the table doesn't show.
    RosterMismatch { en: "The server listed someone who isn't at this table; your message wasn't sent to them", de: "Der Server hat jemanden genannt, der nicht an diesem Tisch sitzt; deine Nachricht ging nicht an diese Person" },
    /// A send that failed for good.
    SendFailed { en: "Not sent", de: "Nicht gesendet" },
    /// A send refused: not a member, or not allowed.
    SendNotAllowed { en: "You can't write to this conversation", de: "Du kannst in diesem Gespräch nicht schreiben" },
    /// A send refused: too long.
    SendTooLarge { en: "That message is too long", de: "Diese Nachricht ist zu lang" },
    /// A send refused: the recipient holds too many messages.
    SendRecipientFull { en: "They have too many messages waiting; try again later", de: "Bei ihnen warten zu viele Nachrichten; versuch es später noch einmal" },
    /// A send refused: this computer's clock is off.
    SendStale { en: "Your computer's clock is off, so the message was refused", de: "Die Uhr deines Computers geht falsch, deshalb wurde die Nachricht abgelehnt" },
    /// A send refused: the gateway has no chat.
    ChatOff { en: "This server has no chat", de: "Dieser Server hat keinen Chat" },
    /// A send refused for a reason this build doesn't know.
    SendRefused { en: "The server refused the message", de: "Der Server hat die Nachricht abgelehnt" },
    /// An unknown command.
    UnknownCommand { en: "Unknown command /{0} — /help lists them", de: "Unbekannter Befehl /{0} — /help listet alle auf" },
    /// A command that needs a name.
    MissingName { en: "/{0} needs a name", de: "/{0} braucht einen Namen" },
    /// A command that needs a message.
    MissingText { en: "/{0} needs a message", de: "/{0} braucht eine Nachricht" },
    /// /t away from a table.
    NoTable { en: "You aren't seated at a table", de: "Du sitzt an keinem Tisch" },
    /// /team at a table without teams.
    NoTeam { en: "This table has no teams", de: "Dieser Tisch hat keine Teams" },
    /// /g with no group.
    NoGroup { en: "There is no group to write to", de: "Es gibt keine Gruppe, an die du schreiben kannst" },
    /// /1–/9 past the groups there are.
    NoGroupNumber { en: "You have no group number {0}", de: "Du hast keine Gruppe Nummer {0}" },
    /// /r before anyone whispered.
    NoReply { en: "Nobody has whispered you yet", de: "Dir hat noch niemand geflüstert" },
    /// A name that matches nobody.
    NoSuchPlayer { en: "Nobody called “{0}” among your friends, recent whispers or this table", de: "Niemand namens „{0}“ unter deinen Freunden, letzten Flüsterpartnern oder an diesem Tisch" },
    /// A name that matches several people.
    AmbiguousName { en: "“{0}” could be: {1}", de: "„{0}“ könnte sein: {1}" },
    /// A guest tried something only registered players have.
    GuestsCannot { en: "Guests have no friends list or groups", de: "Gäste haben keine Freundesliste und keine Gruppen" },
    /// Right-clicking a card whose identity isn't the player's to share.
    PrivateCard { en: "That card's identity isn't yours to share", de: "Die Identität dieser Karte darfst du nicht teilen" },
    /// Right-clicking a card only this player can see.
    OwnCardHint { en: "You're naming a card only you can see", de: "Du nennst eine Karte, die nur du sehen kannst" },
    /// The divider a rematch room begins with.
    NewTable { en: "New table", de: "Neuer Tisch" },
    /// The gateway: someone joined the group.
    MemberJoined { en: "{0} joined the group (server notice)", de: "{0} ist der Gruppe beigetreten (Servernachricht)" },
    /// The gateway: someone left the group.
    MemberLeft { en: "{0} left the group (server notice)", de: "{0} hat die Gruppe verlassen (Servernachricht)" },
    /// The gateway: the group was renamed.
    GroupRenamed { en: "The group is now called {0} (server notice)", de: "Die Gruppe heißt jetzt {0} (Servernachricht)" },
    /// The gateway: the group has a new owner.
    OwnerNow { en: "{0} owns the group now (server notice)", de: "{0} gehört die Gruppe jetzt (Servernachricht)" },
    /// A table invite was sent.
    Invited { en: "Invited {0}", de: "{0} eingeladen" },
    /// A table invite was declined.
    InviteDeclined { en: "{0} declined", de: "{0} hat abgelehnt" },
    /// `/who`: one member.
    WhoLine { en: "{0} · {1} devices", de: "{0} · {1} Geräte" },
    /// `/who` in a conversation with no member list.
    WhoNobody { en: "Nobody else is in this conversation", de: "Niemand sonst ist in diesem Gespräch" },
    /// `/clear` done.
    Cleared { en: "History cleared on this device", de: "Verlauf auf diesem Gerät gelöscht" },
    /// `/mute` done.
    Muted { en: "This conversation is muted", de: "Dieses Gespräch ist stummgeschaltet" },
    /// `/unmute` done.
    Unmuted { en: "This conversation is no longer muted", de: "Dieses Gespräch ist nicht mehr stummgeschaltet" },
    /// `/ignore` done.
    Ignoring { en: "Hiding {0}'s lines on this device", de: "Die Zeilen von {0} werden auf diesem Gerät ausgeblendet" },
    /// `/unignore` done.
    NotIgnoring { en: "Showing {0}'s lines again", de: "Die Zeilen von {0} werden wieder angezeigt" },
    /// `/nick` set.
    NickSet { en: "{0} is now shown as {1} on this device", de: "{0} wird auf diesem Gerät jetzt als {1} angezeigt" },
    /// `/nick` cleared.
    NickCleared { en: "{0} is shown by their own name again", de: "{0} wird wieder mit dem eigenen Namen angezeigt" },
    /// Several hidden lines from ignored players.
    HiddenLines { en: "{0} hidden messages", de: "{0} ausgeblendete Nachrichten" },
    /// One hidden line from an ignored player.
    HiddenLine { en: "1 hidden message", de: "1 ausgeblendete Nachricht" },
    /// The encryption line under a conversation.
    EncryptedTo { en: "Encrypted to {0} players ({1} devices)", de: "Verschlüsselt an {0} Personen ({1} Geräte)" },
    /// A whisper's lock hover.
    EncryptedWith { en: "End-to-end encrypted · {0} ({1} devices)", de: "Ende-zu-Ende-verschlüsselt · {0} ({1} Geräte)" },
    /// The notice's first part (§11.8).
    NoticeBody { en: "Messages are end-to-end encrypted. Only the people in a conversation can read them, on the devices they use Baylee on, as long as those devices are really theirs: compare safety numbers to be sure. The server passes messages on and keeps them, encrypted, for up to {0} days until they are delivered. It cannot read them. It does see who writes to whom and when, who is in a group and its name, and who is online. Table chat goes to the players seated at the table; spectators and computer players never receive it. If you report a message, you send that one message to the moderators. Anyone who can use your device can read the chat history kept on it.", de: "Nachrichten sind Ende-zu-Ende-verschlüsselt. Nur die Personen in einem Gespräch können sie lesen, auf den Geräten, auf denen sie Baylee nutzen – solange diese Geräte wirklich ihre sind: Vergleicht die Sicherheitsnummern, um sicherzugehen. Der Server leitet Nachrichten weiter und bewahrt sie verschlüsselt bis zu {0} Tage auf, bis sie zugestellt sind. Lesen kann er sie nicht. Er sieht aber, wer wem wann schreibt, wer in einer Gruppe ist und wie sie heißt, und wer online ist. Tischchat geht an die Personen, die am Tisch sitzen; Zuschauer und Computerspieler bekommen ihn nie. Wenn du eine Nachricht meldest, schickst du genau diese eine Nachricht an die Moderation. Wer dein Gerät benutzen kann, kann den darauf gespeicherten Chatverlauf lesen." },
    /// Where the keys are: the keychain.
    KeysInKeychain { en: "Your keys are kept in this computer's keychain.", de: "Deine Schlüssel liegen im Schlüsselbund dieses Computers." },
    /// Where the keys are: a file.
    KeysInFile { en: "Your keys are kept in a file only your user account can read (the system keychain was not available).", de: "Deine Schlüssel liegen in einer Datei, die nur dein Benutzerkonto lesen kann (der Schlüsselbund des Systems war nicht verfügbar)." },
    /// Where the keys are: memory.
    KeysInMemory { en: "Your keys are kept in memory only and are lost when Baylee closes.", de: "Deine Schlüssel liegen nur im Arbeitsspeicher und gehen verloren, wenn Baylee geschlossen wird." },
    /// The safety number's instructions.
    SafetyCompare { en: "Compare these numbers with {0}, in person or by voice. If they match, mark as verified.", de: "Vergleiche diese Nummern mit {0}, persönlich oder per Sprache. Wenn sie übereinstimmen, markiere sie als bestätigt." },
    /// The safety number's state: verified.
    SafetyVerified { en: "Verified", de: "Bestätigt" },
    /// The safety number's state: not verified.
    SafetyNotVerified { en: "Not verified", de: "Nicht bestätigt" },
    /// A friend request arrived.
    ToastFriendRequest { en: "{0} wants to be friends", de: "{0} möchte befreundet sein" },
    /// A friend request was accepted.
    ToastFriendAccepted { en: "{0} accepted your friend request", de: "{0} hat deine Freundschaftsanfrage angenommen" },
    /// An invite to join a table.
    ToastJoinInvite { en: "{0} invites you to {1}", de: "{0} lädt dich zu {1} ein" },
    /// An invite to watch a table.
    ToastWatchInvite { en: "{0} invites you to watch {1}", de: "{0} lädt dich ein, bei {1} zuzuschauen" },
    /// A group invite.
    ToastGroupInvite { en: "{0} invites you to the group {1}", de: "{0} lädt dich in die Gruppe {1} ein" },
    /// A group invite whose inviter the event doesn't name.
    ToastGroupInvited { en: "You're invited to the group {0}", de: "Du bist in die Gruppe {0} eingeladen" },
    /// A report was sent.
    ToastReported { en: "Reported. Thank you.", de: "Gemeldet. Danke." },
    /// A whisper arrived (with text).
    ToastWhisper { en: "{0} whispers: {1}", de: "{0} flüstert: {1}" },
    /// A whisper arrived (toasts without text).
    ToastWhisperQuiet { en: "{0} whispered you", de: "{0} hat dir geflüstert" },
    /// A mention arrived.
    ToastMention { en: "{0} mentioned you", de: "{0} hat dich erwähnt" },
    /// A friend came online.
    ToastOnline { en: "{0} is online", de: "{0} ist online" },
    /// This account has as many devices as it may.
    ToastTooManyDevices { en: "This account already has the most chat devices it may; remove one in Settings → Chat & privacy", de: "Dieses Konto hat schon die meisten Chat-Geräte, die es haben darf; entferne eines unter Einstellungen → Chat & Privatsphäre" },
    /// The device couldn't be registered.
    ToastRegisterFailed { en: "Chat couldn't start on this device", de: "Der Chat konnte auf diesem Gerät nicht starten" },
    /// This device was removed from the account.
    ToastDeviceRemoved { en: "This device was removed from your chat; its history was deleted", de: "Dieses Gerät wurde aus deinem Chat entfernt; sein Verlauf wurde gelöscht" },
    /// The server refused the request the player made.
    ToastRefused { en: "The server refused that", de: "Der Server hat das abgelehnt" },
    /// The player was not found.
    ToastNoSuchPlayer { en: "There is no player {0}", de: "Es gibt keinen Spieler {0}" },
    /// Too many requests.
    ToastSlowDown { en: "Too many at once; try again later", de: "Zu viele auf einmal; versuch es später noch einmal" },
    /// Status: online.
    StatusOnline { en: "Online", de: "Online" },
    /// Status: away.
    StatusAway { en: "Away", de: "Abwesend" },
    /// Status: do not disturb.
    StatusDnd { en: "Do not disturb", de: "Nicht stören" },
    /// Status: appear offline.
    StatusInvisible { en: "Appear offline", de: "Offline erscheinen" },
    /// The hint under Appear offline.
    InvisibleHint { en: "Messages you send show that you're online", de: "Nachrichten, die du sendest, zeigen, dass du online bist" },
    /// Presence: offline.
    PlaceOffline { en: "Offline", de: "Offline" },
    /// Presence: in the lobby.
    PlaceLobby { en: "In the lobby", de: "In der Lobby" },
    /// Presence: in a waiting room.
    PlaceRoom { en: "Waiting · {0}", de: "Wartet · {0}" },
    /// Presence: playing.
    PlacePlaying { en: "Playing · {0}", de: "Spielt · {0}" },
    /// Presence: watching.
    PlaceWatching { en: "Watching · {0}", de: "Schaut zu · {0}" },
    /// Presence: in the deck builder.
    PlaceBuilding { en: "Building a deck", de: "Baut ein Deck" },
    /// Presence: playing the house AI.
    PlaceHouse { en: "Playing the house AI", de: "Spielt gegen die Haus-KI" },
    /// The All tab.
    TabAll { en: "All", de: "Alle" },
    /// The Table tab.
    TabTable { en: "Table", de: "Tisch" },
    /// The Team tab.
    TabTeam { en: "Team", de: "Team" },
    /// The friends drawer's sections.
    SectionRequests { en: "Requests", de: "Anfragen" },
    /// The online section.
    SectionOnline { en: "Online", de: "Online" },
    /// The groups section.
    SectionGroups { en: "Groups", de: "Gruppen" },
    /// The offline section.
    SectionOffline { en: "Offline", de: "Offline" },
    /// The blocked section.
    SectionBlocked { en: "Blocked", de: "Blockiert" },
    /// `/help`'s first line.
    HelpIntro { en: "Commands (type /help and a command for more):", de: "Befehle (gib /help und einen Befehl ein für mehr):" },
    /// /t.
    HelpTable { en: "/t text — to the table (also /s, /say, /table)", de: "/t Text — an den Tisch (auch /s, /say, /table)" },
    /// /team.
    HelpTeam { en: "/team text — to your team (also /p, /party)", de: "/team Text — an dein Team (auch /p, /party)" },
    /// /w.
    HelpWhisper { en: "/w name text — whisper; without text, write to them from now on (also /whisper, /tell, /msg)", de: "/w Name Text — flüstern; ohne Text ab jetzt an diese Person schreiben (auch /whisper, /tell, /msg)" },
    /// /r.
    HelpReply { en: "/r text — whisper to whoever whispered you last (also /reply)", de: "/r Text — der Person flüstern, die dir zuletzt geflüstert hat (auch /reply)" },
    /// /g.
    HelpGroup { en: "/g text — to the open group, else the last one (also /group); /1 … /9 to the Nth group", de: "/g Text — an die offene Gruppe, sonst die letzte (auch /group); /1 … /9 an die N-te Gruppe" },
    /// /me.
    HelpEmote { en: "/me action — an emote (also /emote, /e)", de: "/me Handlung — eine Aktion (auch /emote, /e)" },
    /// /friend.
    HelpFriend { en: "/friend name — send a friend request (also /f, /add); /unfriend name (also /remove)", de: "/friend Name — Freundschaftsanfrage senden (auch /f, /add); /unfriend Name (auch /remove)" },
    /// /block.
    HelpBlock { en: "/block name, /unblock name — block on the server", de: "/block Name, /unblock Name — auf dem Server blockieren" },
    /// /ignore.
    HelpIgnore { en: "/ignore name, /unignore name — hide their lines on this device (also /mute name, /unmute name)", de: "/ignore Name, /unignore Name — ihre Zeilen auf diesem Gerät ausblenden (auch /mute Name, /unmute Name)" },
    /// /invite.
    HelpInvite { en: "/invite name — to your waiting room, or to watch your table (also /inv)", de: "/invite Name — in deinen Warteraum oder zum Zuschauen an deinem Tisch (auch /inv)" },
    /// /watch.
    HelpWatch { en: "/watch name — watch a friend's game (also /spectate)", de: "/watch Name — das Spiel eines Freundes ansehen (auch /spectate)" },
    /// Status commands.
    HelpStatus { en: "/online, /away (/afk), /dnd (/busy), /invisible (/offline) — your status", de: "/online, /away (/afk), /dnd (/busy), /invisible (/offline) — dein Status" },
    /// /friends.
    HelpFriends { en: "/friends — the friends list (also /social)", de: "/friends — die Freundesliste (auch /social)" },
    /// /who.
    HelpWho { en: "/who — who is in this conversation", de: "/who — wer in diesem Gespräch ist" },
    /// /nick.
    HelpNick { en: "/nick name [nickname] — set or clear a nickname on this device", de: "/nick Name [Spitzname] — einen Spitznamen auf diesem Gerät setzen oder entfernen" },
    /// /leave.
    HelpLeave { en: "/leave — leave the open group", de: "/leave — die offene Gruppe verlassen" },
    /// /clear.
    HelpClear { en: "/clear — clear this conversation on this device", de: "/clear — dieses Gespräch auf diesem Gerät leeren" },
    /// /mute.
    HelpMute { en: "/mute, /unmute — this conversation's notifications", de: "/mute, /unmute — die Benachrichtigungen dieses Gesprächs" },
    /// /help.
    HelpHelp { en: "/help [command] — this list (also /?, /commands)", de: "/help [Befehl] — diese Liste (auch /?, /commands)" },
    /// /leave asks first.
    LeaveConfirm { en: "Leave {0}? Type /leave again to confirm", de: "{0} verlassen? Gib /leave noch einmal ein, um zu bestätigen" },
    /// The composer's hint.
    ComposeHint { en: "# card · @ player", de: "# Karte · @ Person" },
    /// The composer's placeholder.
    WriteTo { en: "Write to {0}…", de: "An {0} schreiben…" },
    /// The target chip.
    ToTarget { en: "To {0}", de: "An {0}" },
    /// The All tab's prefix for a whisper received.
    WhisperFrom { en: "{0} → you", de: "{0} → dir" },
    /// The All tab's prefix for a whisper sent.
    WhisperTo { en: "you → {0}", de: "du → {0}" },
}

impl ChatPhrase {
    /// This phrase with its placeholders filled in, left to right; a
    /// placeholder with no value is left standing.
    #[must_use]
    pub fn fill(self, lang: Lang, args: &[&str]) -> String {
        let mut text = self.text(lang).to_string();
        for (index, arg) in args.iter().enumerate() {
            text = text.replace(&format!("{{{index}}}"), arg);
        }
        text
    }

    /// The placeholders this phrase carries, as their indices.
    #[must_use]
    pub fn slots(self, lang: Lang) -> Vec<usize> {
        let text = self.text(lang);
        (0..10)
            .filter(|i| text.contains(&format!("{{{i}}}")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ChatPhrase;
    use crate::i18n::Lang;

    /// A translation keeps every placeholder: a dropped `{0}` is a name or
    /// a count the player never sees.
    #[test]
    fn a_translation_keeps_every_placeholder_it_was_given() {
        for phrase in ChatPhrase::ALL {
            let english = phrase.slots(Lang::En);
            for lang in Lang::ALL {
                assert_eq!(phrase.slots(lang), english, "{phrase:?} in {lang:?}");
                assert!(!phrase.text(lang).is_empty(), "{phrase:?} in {lang:?}");
            }
        }
    }

    /// No straight apostrophes in German: the shell's typography uses the
    /// typographic ones, and a chat phrase reads like the rest.
    #[test]
    fn german_quotes_are_typographic() {
        for phrase in ChatPhrase::ALL {
            assert!(!phrase.text(Lang::De).contains('"'), "{phrase:?}");
        }
    }
}
