### One thing the chat says.

# Local encrypted vault.
VaultTitle = Encrypted local vault

# Local encrypted vault.
VaultExplain = The account password unlocks local chat keys. After a password reset, use the original vault password. An optional new password rewraps the same keys. Creating or importing requires repeating the chosen password in the last field. Imports preserve their sources. A new vault creates a new chat device; previous keys and history remain untouched. Guests and browser tabs keep keys in memory only.

# Local encrypted vault.
VaultOpen = Unlock chat

# Local encrypted vault.
VaultLock = Lock chat

# Local encrypted vault.
VaultPassword = Vault password

# Repeated new wrapping password prevents accidental lockout.
VaultPasswordAgain = Repeat new vault password

# Two copies of the new password differ.
VaultPasswordMismatch = The two new vault passwords differ.

# Local encrypted vault.
VaultNewPassword = New vault password (optional)

# Local encrypted vault.
VaultUnlock = Unlock / change vault password

# Local encrypted vault.
VaultImportFile = Import legacy key file

# Local encrypted vault.
VaultImportKeychain = Import from system keychain (may ask for access)

# Local encrypted vault.
VaultCreate = Create new encrypted vault

# Local encrypted vault.
VaultConfirmCreate = Confirm: create a new chat device

# Local encrypted vault.
VaultLater = Continue without chat · Esc

# Local encrypted vault.
VaultWorking = Unlocking… You can continue playing while this finishes.

# Local encrypted vault.
VaultWrongPassword = Cannot unlock: wrong original password or damaged vault. Nothing was replaced.

# Local encrypted vault.
VaultMissing = No encrypted vault or importable key was found. Choose an import or explicitly create a new device.

# Local encrypted vault.
VaultBusy = Another vault operation or Baylee process is still using the vault.

# Local encrypted vault.
VaultExists = Existing vault or chat data found. Unlock or import it; it will not be overwritten.

# Local encrypted vault.
VaultFailed = The vault could not be safely read or saved. Existing data was preserved.

# A line that failed its signature, wrap or body check.
MessageDropped = A message failed its check and was dropped

# A line from a device this client couldn't find.
# Arguments: arg0
UnknownDevice = A message from { $arg0 } came from a device that couldn't be checked and was dropped

# A line addressed somewhere it doesn't belong.
MisaddressedDropped = A message that didn't belong in this conversation was dropped

# A contact added a device.
# Arguments: arg0, arg1, arg2
DeviceAdded = { $arg0 } added a device ({ $arg1 } · { $arg2 })

# A contact removed a device.
# Arguments: arg0
DeviceRemoved = { $arg0 } removed a device

# A verified contact's devices changed.
# Arguments: arg0
VerifyAgain = { $arg0 }'s devices changed · Verify again

# This account gained a device.
# Arguments: arg0, arg1
OwnDeviceAdded = A new device was added to your account: { $arg0 } · { $arg1 }. Not you? Remove it under Settings → Chat & privacy → Your devices; the gateway's admin can reset your password.

# The gateway listed someone the table doesn't show.
RosterMismatch = The server listed someone who isn't at this table; your message wasn't sent to them

# A send that failed for good.
SendFailed = Not sent

# A send refused: not a member, or not allowed.
SendNotAllowed = You can't write to this conversation

# A send refused: too long.
SendTooLarge = That message is too long

# A send refused: the recipient holds too many messages.
SendRecipientFull = They have too many messages waiting; try again later

# A send refused: this computer's clock is off.
SendStale = Your computer's clock is off, so the message was refused

# A send refused: the gateway has no chat.
ChatOff = This server has no chat

# A send refused for a reason this build doesn't know.
SendRefused = The server refused the message

# An unknown command.
# Arguments: arg0
UnknownCommand = Unknown command /{ $arg0 } — /help lists them

# A command that needs a name.
# Arguments: arg0
MissingName = /{ $arg0 } needs a name

# A command that needs a message.
# Arguments: arg0
MissingText = /{ $arg0 } needs a message

# /t away from a table.
NoTable = You aren't seated at a table

# /team at a table without teams.
NoTeam = This table has no teams

# /g with no group.
NoGroup = There is no group to write to

# /1–/9 past the groups there are.
# Arguments: arg0
NoGroupNumber = You have no group number { $arg0 }

# /r before anyone whispered.
NoReply = Nobody has whispered you yet

# A name that matches nobody.
# Arguments: arg0
NoSuchPlayer = Nobody called “{ $arg0 }” among your friends, recent whispers or this table

# A name that matches several people.
# Arguments: arg0, arg1
AmbiguousName = “{ $arg0 }” could be: { $arg1 }

# A guest tried something only registered players have.
GuestsCannot = Guests have no friends list or groups

# Right-clicking a card whose identity isn't the player's to share.
PrivateCard = That card's identity isn't yours to share

# Right-clicking a card only this player can see.
OwnCardHint = You're naming a card only you can see

# The divider a rematch room begins with.
NewTable = New table

# The gateway: someone joined the group.
# Arguments: arg0
MemberJoined = { $arg0 } joined the group (server notice)

# The gateway: someone left the group.
# Arguments: arg0
MemberLeft = { $arg0 } left the group (server notice)

# The gateway: the group was renamed.
# Arguments: arg0
GroupRenamed = The group is now called { $arg0 } (server notice)

# The gateway: the group has a new owner.
# Arguments: arg0
OwnerNow = { $arg0 } owns the group now (server notice)

# A table invite was sent.
# Arguments: arg0
Invited = Invited { $arg0 }

# A table invite was declined.
# Arguments: arg0
InviteDeclined = { $arg0 } declined

# `/who`: one member.
# Arguments: arg0, arg1
WhoLine = { $arg0 } · { $arg1 } devices

# `/who` in a conversation with no member list.
WhoNobody = Nobody else is in this conversation

# `/clear` done.
Cleared = History cleared on this device

# `/mute` done.
Muted = This conversation is muted

# `/unmute` done.
Unmuted = This conversation is no longer muted

# `/ignore` done.
# Arguments: arg0
Ignoring = Hiding { $arg0 }'s lines on this device

# `/unignore` done.
# Arguments: arg0
NotIgnoring = Showing { $arg0 }'s lines again

# `/nick` set.
# Arguments: arg0, arg1
NickSet = { $arg0 } is now shown as { $arg1 } on this device

# `/nick` cleared.
# Arguments: arg0
NickCleared = { $arg0 } is shown by their own name again

# Several hidden lines from ignored players.
# Arguments: arg0
HiddenLines = { $arg0 } hidden messages

# One hidden line from an ignored player.
HiddenLine = 1 hidden message

# The encryption line under a conversation.
# Arguments: arg0, arg1
EncryptedTo = Encrypted to { $arg0 } players ({ $arg1 } devices)

# The encryption line's count of devices: one.
NoteOneDevice = 1 device

# The encryption line's count of devices.
# Arguments: arg0
NoteDevices = { $arg0 } devices

# The encryption line's count of people: one.
NoteOnePerson = 1 person

# The encryption line's count of people.
# Arguments: arg0
NotePeople = { $arg0 } people

# A whisper's lock hover.
# Arguments: arg0, arg1
EncryptedWith = End-to-end encrypted · { $arg0 } ({ $arg1 } devices)

# The notice's first part (§11.8).
# Arguments: arg0
NoticeBody = Messages are end-to-end encrypted to the listed recipient devices. Compare safety numbers through a trusted channel: the gateway supplies first-contact keys and group/table membership, so a malicious operator can misrepresent recipients. Verified peer keys alone do not verify group membership. The server keeps encrypted messages for up to { $arg0 } days until delivery and sees who writes to whom, group names and online status. Reporting sends that message to moderators. Anyone who can use your unlocked device can read its chat history.

# Where the keys are: the keychain.
KeysInKeychain = Your keys are kept in this computer's keychain.

# Password-wrapped random local encryption key.
KeysInPasswordVault = Your keys are encrypted locally. Your account password unlocks the vault; a password reset cannot recover it. Keep your original password until you have changed the vault password too.

# Account-encrypted central archive.
KeysInCentralVault = Your chat keys are encrypted in your Baylee account. One account session uses the shared chat identity; signing in restores its encrypted archive and pending deliveries.

# Local sign-out must not revoke an account-wide chat identity.
CentralDeviceSignOut = Use account sign-out to disconnect this installation. The shared chat identity stays available on your next device.

# Local changes still await the encrypted Central backup.
ArchivePending = Chat backup pending · keep this device online

# A failed backup must never be shown as synchronized.
ArchiveFailed = Chat backup failed · latest changes may exist only on this device

# Every completed local write is in an acknowledged Central manifest.
ArchiveSaved = This device's chat is backed up

# Legacy key file, only available for explicit import.
KeysInFile = Your keys are kept in a file only your user account can read (the system keychain was not available).

# Where the keys are: memory.
KeysInMemory = Your keys are kept in memory only and are lost when Baylee closes.

# The safety number's instructions.
# Arguments: arg0
SafetyCompare = Compare these numbers with { $arg0 }, in person or by voice. If they match, mark as verified.

# The safety number's state: verified.
SafetyVerified = Verified

# The safety number's state: not verified.
SafetyNotVerified = Not verified

# A friend request arrived.
# Arguments: arg0
ToastFriendRequest = { $arg0 } wants to be friends

# A friend request was accepted.
# Arguments: arg0
ToastFriendAccepted = { $arg0 } accepted your friend request

# An invite to join a table.
# Arguments: arg0, arg1
ToastJoinInvite = { $arg0 } invites you to { $arg1 }

# An invite to watch a table.
# Arguments: arg0, arg1
ToastWatchInvite = { $arg0 } invites you to watch { $arg1 }

# A group invite.
# Arguments: arg0, arg1
ToastGroupInvite = { $arg0 } invites you to the group { $arg1 }

# A group invite whose inviter the event doesn't name.
# Arguments: arg0
ToastGroupInvited = You're invited to the group { $arg0 }

# A report was sent.
ToastReported = Reported. Thank you.

# A whisper arrived (with text).
# Arguments: arg0, arg1
ToastWhisper = { $arg0 } whispers: { $arg1 }

# A whisper arrived (toasts without text).
# Arguments: arg0
ToastWhisperQuiet = { $arg0 } whispered you

# A mention arrived.
# Arguments: arg0
ToastMention = { $arg0 } mentioned you

# A friend came online.
# Arguments: arg0
ToastOnline = { $arg0 } is online

# This account has as many devices as it may.
ToastTooManyDevices = This account already has the most chat devices it may; remove one in Settings → Chat & privacy

# The device couldn't be registered.
ToastRegisterFailed = Chat couldn't start on this device

# This device was removed from the account.
ToastDeviceRemoved = This device was removed from your chat; its history was deleted

# The server refused the request the player made.
ToastRefused = The server refused that

# The player was not found.
# Arguments: arg0
ToastNoSuchPlayer = There is no player { $arg0 }

# Too many requests.
ToastSlowDown = Too many at once; try again later

# Status: online.
StatusOnline = Online

# Status: away.
StatusAway = Away

# Status: do not disturb.
StatusDnd = Do not disturb

# Status: appear offline.
StatusInvisible = Appear offline

# The hint under Appear offline.
InvisibleHint = Messages you send show that you're online

# Presence: offline.
PlaceOffline = Offline

# Presence: in the lobby.
PlaceLobby = In the lobby

# Presence: in a waiting room.
# Arguments: arg0
PlaceRoom = Waiting · { $arg0 }

# Presence: playing.
# Arguments: arg0
PlacePlaying = Playing · { $arg0 }

# Presence: watching.
# Arguments: arg0
PlaceWatching = Watching · { $arg0 }

# Presence: in the deck builder.
PlaceBuilding = Building a deck

# Presence: playing the house AI.
PlaceHouse = Playing the house AI

# The All tab.
TabAll = All

# The Global Lobby tab.
TabGlobal = Lobby

# The Table tab.
TabTable = Table

# The Team tab.
TabTeam = Team

# The friends drawer's sections.
SectionRequests = Requests

# The online section.
SectionOnline = Online

# The groups section.
SectionGroups = Groups

# The offline section.
SectionOffline = Offline

# The blocked section.
SectionBlocked = Blocked

# `/help`'s first line.
HelpIntro = Commands (type /help and a command for more):

# /t.
HelpTable = /t text — to the table (also /s, /say, /table)

# /team.
HelpTeam = /team text — to your team (also /p, /party)

# /w.
HelpWhisper = /w name text — whisper; without text, write to them from now on (also /whisper, /tell, /msg)

# /r.
HelpReply = /r text — whisper to whoever whispered you last (also /reply)

# /g.
HelpGroup = /g text — to the open group, else the last one (also /group); /1 … /9 to the Nth group

# /me.
HelpEmote = /me action — an emote (also /emote, /e)

# /friend.
HelpFriend = /friend name — send a friend request (also /f, /add); /unfriend name (also /remove)

# /block.
HelpBlock = /block name, /unblock name — block on the server

# /ignore.
HelpIgnore = /ignore name, /unignore name — hide their lines on this device (also /mute name, /unmute name)

# /invite.
HelpInvite = /invite name — to your waiting room, or to watch your table (also /inv)

# /watch.
HelpWatch = /watch name — watch a friend's game (also /spectate)

# Status commands.
HelpStatus = /online, /away (/afk), /dnd (/busy), /invisible (/offline) — your status

# /friends.
HelpFriends = /friends — the friends list (also /social)

# /who.
HelpWho = /who — who is in this conversation

# /nick.
HelpNick = /nick name [nickname] — set or clear a nickname on this device

# /leave.
HelpLeave = /leave — leave the open group

# /clear.
HelpClear = /clear — clear this conversation on this device

# /mute.
HelpMute = /mute, /unmute — this conversation's notifications

# /help.
HelpHelp = /help [command] — this list (also /?, /commands)

# /leave asks first.
# Arguments: arg0
LeaveConfirm = Leave { $arg0 }? Type /leave again to confirm

# The composer's hint.
ComposeHint = # card · @ player

# The composer's placeholder.
# Arguments: arg0
WriteTo = Write to { $arg0 }…

# The target chip.
# Arguments: arg0
ToTarget = To { $arg0 }

# The All tab's prefix for a whisper received.
# Arguments: arg0
WhisperFrom = From { $arg0 }:

# The All tab's prefix for a whisper sent.
# Arguments: arg0
WhisperTo = To { $arg0 }:

# The Log tab.
TabLog = Log

# The Private tab.
TabPrivate = Private

# The Group tab.
TabGroup = Group

# The chat window and its door.
ChatTitle = Chat

# The chat door's hint with its count.
# Arguments: arg0
ChatDoor = Chat · { $arg0 } unread

# The friends drawer.
FriendsTitle = Friends

# The drawer's search box.
FindOrAdd = Name#tag — find or add

# Add a typed handle.
# Arguments: arg0
AddHandle = Add { $arg0 }

# Accept a request.
Accept = Accept

# Decline a request.
Decline = Decline

# Withdraw a request.
Withdraw = Undo

# A request this player sent.
RequestSent = sent

# Whisper someone.
WhisperButton = Whisper

# Invite a friend to this table.
InviteToTable = Invite to my table

# Watch a friend's table.
WatchButton = Watch

# Join a friend's table.
JoinButton = Join

# Put an invite off.
NotNow = Not now

# Block someone.
BlockButton = Block

# Unblock someone.
UnblockButton = Unblock

# Remove a friend.
RemoveFriend = Remove friend

# Set a nickname.
NicknameButton = Nickname…

# Open the safety number.
SafetyButton = Safety number…

# Make a group.
NewGroupButton = New group…

# Rename a group.
RenameGroup = Rename…

# Leave a group.
LeaveGroup = Leave group

# Invite a friend to a group.
InviteToGroup = Invite a friend…

# Delete a group.
DeleteGroup = Delete group

# A group's members and how many are online.
# Arguments: arg0, arg1
GroupCount = { $arg0 } · { $arg1 } online

# The Group tab with no group.
NoGroupYet = No group yet

# The new group's name box.
GroupNameLabel = Group name

# The nickname box.
# Arguments: arg0
NicknameLabel = Nickname for { $arg0 }

# The group invite box.
InviteLabel = Friend to invite (Name#tag)

# Close a dialog.
Done = Done

# Reset a dialog.
Reset = Reset

# Cancel.
CancelButton = Cancel

# Confirm a small box.
OkButton = OK

# The filter dialog's title.
# Arguments: arg0
FilterTitle = { $arg0 } · what this tab shows

# A filter kind.
KindGlobal = Lobby chat

# A filter kind.
KindLog = Game log

# A filter kind.
KindTable = Table chat

# A filter kind.
KindTeam = Team chat

# A filter kind.
KindWhispers = Whispers

# A filter kind.
KindGroups = Groups

# A filter kind.
KindNotices = Notices

# A filter kind.
KindPresence = Friends online/offline

# A filter option.
KindTimestamps = Timestamps

# Under the filter dialog's boxes.
FilterNote = A line is shown on every tab that ticks its kind. The tab's name and where you write stay.

# The Global Lobby tab, empty.
EmptyGlobal = Say hello to everyone online in the lobby!

# The Table tab, empty, in a game.
EmptyTable = Nothing said yet. What you write here goes to the players at this table, encrypted.

# The Table tab, empty, in a waiting room.
EmptyRoom = Say hello — the players who sit down here will read it.

# The Team tab, empty.
EmptyTeam = No team talk yet. Only your team reads this tab.

# The Private tab, empty.
EmptyPrivate = No whispers yet. Type /w Name#tag, or click a name in the chat.

# The Group tab with no group.
EmptyGroupNone = You're in no group yet.

# The Group tab, empty.
# Arguments: arg0
EmptyGroup = Nobody has written in { $arg0 } yet.

# The All tab, empty.
EmptyAll = Quiet so far.

# A tab whose filter admits nothing.
EmptyFiltered = This tab shows nothing. ⚙ picks what it shows.

# The placeholder for a whisper.
# Arguments: arg0
WriteWhisper = Whisper { $arg0 }…

# The placeholder with no whisper partner.
TypeWhisper = Type /w Name#tag…

# Copy a line.
CopyButton = Copy

# Answer a whisper.
ReplyButton = Reply

# Add someone as a friend.
AddFriendButton = Add friend

# Hide someone's lines.
IgnoreButton = Ignore

# Report a message.
ReportMessage = Report message

# Send a failed line again.
RetryButton = Retry

# Remove a failed line or a device.
RemoveButton = Remove

# Close the encryption notice.
GotIt = Got it

# Mark a safety number verified.
MarkVerified = Mark as verified

# Drop the verified mark.
ClearVerified = Clear verification

# The encryption sheet's title.
EncryptionTitle = End-to-end encrypted

# The safety number's heading.
# Arguments: arg0
SafetyWith = Safety number with { $arg0 }

# A contact's devices.
# Arguments: arg0, arg1
DevicesOf = { $arg0 }'s devices: { $arg1 }

# A safety number not known yet.
NoKeysYet = Their keys aren't known yet: write to them once.

# Settings → Chat & privacy.
PrivacyTitle = Chat & privacy

# A setting.
WhoMayWhisper = Who may whisper me

# A setting's value.
WhispersFriends = Friends only

# A setting's value.
WhispersTables = Friends and players at my table

# A setting.
AcceptRequests = Accept friend requests

# A setting.
ShareTable = Show friends which table I'm at

# A setting.
KeepHistory = Keep chat history on this device

# History off.
HistoryOff = Off

# History kept.
# Arguments: arg0
HistoryDays = { $arg0 } days

# A setting.
SoundsLabel = Sounds

# A sound.
SoundMention = Mentions

# A sound.
SoundFriendOnline = A friend comes online

# A setting.
ToastTextLabel = Show message text in toasts

# A settings list.
YourDevices = Your devices

# A settings list's mark.
ThisDevice = This device

# Remove this device from the chat.
ForgetDevice = Forget this device

# Confirm forgetting this device.
ForgetConfirm = Removes this device's chat keys and history here and on the server. Press again to confirm.

# A settings list.
BlockedPlayers = Blocked players

# A switch that is on.
OnLabel = On

# A switch that is off.
OffLabel = Off

# A group's members.
Members = Members

# Marks this player in a list.
YouMark = (you)

# Marks an invited player.
InvitedMark = (invited)

# Marks the group's owner.
OwnerMark = owner

# Invite a friend to watch this table.
InviteToWatch = Invite to watch

# Invites need a table.
NotAtTable = Sit at a table to invite friends

# The chat's link is down.
ChatOffline = Chat is reconnecting…

# The drawer's recent players.
RecentPlayers = Recent players

# The drawer's status menu.
# Arguments: arg0
StatusLabel = Status: { $arg0 }

# A group invite in the drawer.
# Arguments: arg0, arg1
GroupInviteFrom = { $arg0 } · from { $arg1 }

# Online friends at a running table.
SectionPlaying = Playing

# Online friends seated at a waiting table.
SectionAtTable = At a table

# Online friends in the lobby or elsewhere.
SectionInLobby = In the lobby

# The divider above the first line that was unread when a tab opened.
UnreadMarker = New

# The drawer's door to the settings.
PrivacyButton = Chat & privacy…
