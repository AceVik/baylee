<!-- version: draft -->
<!-- updated: 2026-11-25 -->

# Terms of Use for Baylee

Gateway at https://gateway.baylee.app · Version [VERSION] · As of [DATE]

**Draft for the open beta, not in force.** Before publishing, every [OPEN: …] and [ASSUMPTION: …] is settled and removed, and [NAME], [E-MAIL], [VERSION] and [DATE] are filled in.

This is an English translation. The German version is binding; if the two differ, the German version prevails.

## What is new in this version

- You sign in with your Baylee account at Baylee Central. Baylee Central manages the username and password; the gateway no longer stores any password (sections 1, 4 and 16.3).
- The closed beta is over, and with it the beta keys. There are no guest accounts on this gateway any more; to play you need a Baylee account (sections 3.5 and 4).
- The minimum age is now 13, here as at Baylee Central (section 5).
- Friends, groups, personal messages and your online status run through Baylee Central. Table, team and lobby chat stay on the gateway (section 9a). The gateway has deleted the friendships, requests and groups from before Baylee Central (section 9a.3).
- New are ranked games and the daily and weekly tasks, which Baylee Central arranges and the gateway plays out (sections 3.1, 9.8 and 16.3), ban lists of your own (sections 6.1 and 16.3), and sleeves and playmats from your inventory at Baylee Central (section 16.3).
- Bug reports go to Baylee Central instead of the feedback service; reported chat messages still go to the feedback service (section 10). [OPEN: Before publishing, set `BAYLEE_REPORTS_TO=central` and `BAYLEE_DIRECT_REPORT_KEY` on the server; the code for it is done.]
- Backups are now kept encrypted in my storage on Google Drive; undelivered messages stay in them for at most 14 days, everything else for at most 12 months (sections 16.3, 16.6 and 16.8).
- The gateway's address is now https://gateway.baylee.app; the former address https://baylee.acevik.de leads to the same gateway.

## The essentials in brief

This summary is a reading aid; the numbered sections are binding.

- Baylee is a free, private, non-commercial fan project in beta. It may be unavailable, contain errors and lose data.
- To play, you sign in with your Baylee account, which you create at Baylee Central; its own terms and privacy policy apply to that. The gateway receives from Baylee Central only who you are and your display name, never your password.
- Play fair, in chat too: no cheating, no exploiting bugs (report them), no harassment, no outside bots other than the AI seats the app itself offers.
- Every game on the gateway is recorded in full, and the record is not deleted. I use it internally to find bugs, for balancing and to train the house AI. You can watch your games as replays; your fellow players see in them what you saw (your hand, for example) unless you untick that. I do not sell data, show ads or track you.
- Messages to your table, your team and individual players are end-to-end encrypted: I cannot read their content unless someone reports a message to me. The lobby chat is public and not encrypted.
- You can delete your data on this gateway at any time in the client. You delete your Baylee account at Baylee Central.

## 1. Definitions

- **Baylee**: the project as a whole, i.e. gateway, Baylee Central, client, feedback service and source code.
- **Gateway**: the game server at https://gateway.baylee.app (also at the former address https://baylee.acevik.de) that I operate, with the game processes that run games there, the start page at https://gateway.baylee.app/ and, where offered, the web version of the client at https://gateway.baylee.app/play/.
- **Baylee Central**: the account service at https://baylee.app where you keep your Baylee account. I operate it too. It is governed by its own terms of use and its own privacy policy (https://baylee.app/en/privacy).
- **Client**: the program you play with (desktop, mobile device or browser).
- **Feedback service**: the service at https://feedback.baylee.app that receives and stores reported chat messages, messages from the contact form and the anonymous count, and through which the administrators manage the gateway.
- **Administrators**: me and the people I entrust with operation, bug fixing or moderation and bind to confidentiality. At present that is only me.
- **Baylee account**: your account at Baylee Central. **Account**: the account the gateway keeps for your Baylee account and links to it. **Legacy account**: an account created on the gateway with a username and password before 22 October 2026 (section 4.6).
- **Table**: a room in the lobby where a game is prepared and played. **Seat**: a place at a table. **Spectator**: someone who follows a running game without taking part in it.
- **Ranked game**: a game for which Baylee Central pairs two accounts and whose result counts towards the rating at Baylee Central. **Task**: a daily or weekly task from Baylee Central whose game is played on the gateway.
- **House AI**: the computer opponents and stand-ins the gateway itself provides.
- **AI seat** (also language-model seat): a seat played by a language model. **Seat bridge**: the helper program that connects an AI seat to the language model; except for hosted AI seats it runs on your device. **Hosted AI seat**: an AI seat whose seat bridge and model access I operate (section 8.10).
- **Chair ticket**: a short-lived single-use key with which a seat bridge takes a seat at a table. **Seat token** and **socket ticket**: keys that open a game, lobby, spectator or chat connection.
- **Record**: the complete log of a game (section 9). **Replay**: a view of a game computed afresh from the record (section 9.7). **Report**: a bug, crash or idea report, or a reported chat message (section 10).
- **Chat**: table, team and lobby chat under section 9a. **Device** (in chat): an installation of the client that has its own chat keys for your account.
- **Content**: anything you yourself place on the gateway or send through it, such as names, decks, ban lists, notes, pictures, chat messages and report texts.

## 2. Operator, scope and conclusion of the contract

**2.1 Operator.** The gateway and the feedback service are operated by [NAME] as a private individual, free of charge and not commercially ("I"). You can reach me at [E-MAIL] and through the contact form on the start page. My legal notice (Impressum) is at https://gateway.baylee.app/site/impressum.html; the client links to it.

**2.2 Scope.** These terms govern your use of the gateway and the feedback service. Your Baylee account and everything Baylee Central itself offers (such as friends, groups, personal messages, online status, rankings, tasks and your inventory) are governed by Baylee Central's terms of use and privacy policy. Baylee is open source; anyone can run their own gateway. Other gateways are governed by their own terms, and their operators are responsible for them. If you play offline against the computer without a gateway, only the licences in section 14 apply; if your client sends a report or the anonymous count while doing so, sections 10 and 16 apply to that.

**2.3 Conclusion of the contract.** The contract of use is concluded when you accept these terms in the client with "Accept & continue". The client asks for this when you first sign in to this gateway with your Baylee account and when you sign in after a change to the terms. You can accept only after the text has been shown to its end. For a Baylee account, Baylee Central stores which version you accepted and when (section 16.3).

**2.4 Free of charge.** Use is free. There are no paid features, no in-game purchases and no advertising, and I will not ask you for payment.

**2.5 Relationship to the software licence.** Baylee's software is licensed under the GNU Affero General Public License 3.0 (section 14.4). These terms do not restrict your rights under that licence. They only govern how you use my gateway.

**2.6 Availability of the text.** The client shows you the version in force; the gateway also serves it at https://gateway.baylee.app/terms as a data record. I recommend you keep a copy.

## 3. The service and its beta phase

**3.1 What is offered.** The gateway lets you play an implementation of the rules of a trading card game online: against other people, against the house AI or with AI seats, with a lobby, tables, deck management, ban lists of your own, display of card texts and card images, spectating, replays, and chat at the table and in the lobby. It also plays out the ranked games and the games of tasks that Baylee Central arranges.

**3.2 Beta.** Baylee is in a beta phase. In particular, this means:

- Features may be incomplete or faulty; individual cards or rules may be implemented incorrectly.
- Features may change, be added or be removed.
- Data may be lost or may have to be reset, including decks and settings. I will announce resets in advance where I can.
- Running games may be interrupted by maintenance, restarts or errors.

Keep copies of important decks on your own device using the deck builder's export function.

**3.3 No entitlement to availability.** I provide the gateway voluntarily and without consideration. There is no entitlement to any particular availability, performance or feature (section 11).

**3.4 Versions.** Client and gateway must match the same version. An outdated client may be refused; install the current version in that case.

**3.5 Access.** I may admit, limit or exclude new accounts signing in to this gateway. Existing accounts are not affected. The gateway admits no guests without a Baylee account (section 4.5).

**3.6 Third-party offerings.** Card data and images come from Scryfall, downloads and source code are hosted on GitHub, and for your own AI seats you use providers of your choice. Those offerings are governed by their own terms; I have no influence on them. I choose the provider of a hosted AI seat (section 8.10).

## 4. Baylee account and legacy account

**4.1 Baylee account.** You sign in with your Baylee account, which you create at Baylee Central (https://baylee.app). Baylee Central manages the sign-in name, the password and a voluntary e-mail address; the gateway does not receive them. When you sign in, Baylee Central confirms to the gateway which Baylee account is signing in and tells it the display name with a tag appended ("Name#1a2b") under which others see you. For this the gateway keeps an account of its own, linked to your Baylee account.

**4.2 Names.** Display names, table names, deck names, the names of your ban lists and the names of your AI seats must comply with section 7. In particular, you must not pretend to be another person, me, or a representative of Wizards of the Coast or Scryfall.

**4.3 Personal use.** Your account is personal. You must not sell, rent or give it to others. Several accounts are allowed as long as you do not use them to evade a block, manipulate game results or rankings, or get around limits.

**4.4 Security.** The security of your Baylee account, i.e. password, recovery code and password reset, is governed by Baylee Central's terms. A sign-in at the gateway stays valid for 12 hours and is extended when used; if your session at Baylee Central ends, it ends here too. If you notice misuse of your account, write to me.

**4.5 No guests.** This gateway admits no guests without a Baylee account; to play you need a Baylee account (section 4.1). Should that change, a new version of these terms applies to it.

**4.6 Legacy accounts.** An account created on the gateway with a username and password before 22 October 2026 signs in through Baylee Central and moves there at its first sign-in, taking its settings, decks and their version history along into your Baylee account. On the gateway it remains under its display name and tag and is linked to your Baylee account. The gateway has stored no password since 19 November 2026; what Baylee Central keeps for legacy accounts that have not moved yet, and for how long, is set out in its privacy policy.

**4.7 Deleting your data.** You can delete your data on this gateway yourself at any time: in the client under Settings → "Delete account". Your current sign-in is enough for this. Deleted in one step are the account on this gateway, your sign-ins here, your decks with their version history, your ban lists, your settings, your claims on uploaded pictures, your chat devices, your blocks from before Baylee Central, undelivered messages from and to you, and your archivings of replays; a picture no other account still claims is deleted together with its file. Your seats at all tables are vacated; the house AI plays out a running game for you. What remains afterwards is set out in section 16.10. Your Baylee account remains; you delete it at Baylee Central. [OPEN: Does the gateway delete its data by itself when you delete your Baylee account at Baylee Central? Not shown in the code.] If you cannot use the client, write to me with your display name and tag; I may then ask for proof that the account is yours.

## 5. Minimum age and minors

**5.1** Baylee is meant for people aged 13 and over; younger children must not use the gateway. Baylee Central sets the same minimum age for the Baylee account you need to play. [OPEN: Central's version of 25.11.2026 with 13 years is merged (AceVik/baylee-central#92) and applies with Central's next release; until then Central requires 16.]

**5.2** If you are under 18, you need the agreement of your parents or other legal guardians to these terms.

**5.3** If you are under 16, you may send crash reports, which rely on consent (section 10.5), only with a parent's agreement (Art. 8 GDPR).

**5.4** I do not ask for or check anyone's age. If I learn that an account belongs to a child under the age limit, I may delete it.

## 6. Your content and rights in it

**6.1 What it includes.** Content includes in particular the names of tables and AI seats, table passwords, decks with their name, description and card notes, your ban lists with their name and comments, uploaded sleeves and playmats, your chat messages, and the texts and window pictures of your reports.

**6.2 Your rights stay yours.** You keep all rights in your content. You grant me only the simple (non-exclusive), free, non-transferable right to store it, copy it technically and display it as needed to operate the gateway. For pictures this includes cropping, resizing and re-encoding them, showing them to the players at your table and serving them at their picture address. I store encrypted chat messages only to deliver them, and the lobby chat only in memory; a reported message I may use for moderation (section 10.8). The right ends when the content is deleted; copies in backups and caches disappear when those expire (section 16). Section 10.6 applies in addition to reports.

**6.3 Who sees what.** All signed-in users can find your account by "Name#tag" and see its display name and tag. In the lobby, all signed-in users see the open and running tables with the table name, the host, the names of those seated and their deck names, how long the game has been running, the number of spectators and the provider of a hosted AI seat. Your fellow players see in the game what the rules show them, and your sleeves and playmats. If the host chooses one of your ban lists, everyone at the table sees its name and its cards, never whose it is; others see only that the table has a ban list. If the host allows spectators (the default), any signed-in user can follow a running game, even at a table with a password; spectators see the names of those seated and the open table, no hidden hand and no face-down card. Anyone who sat at a table with you sees in their replays your current display name and tag and the game, and your hidden cards under section 9.7. Nobody else sees the contents, description or notes of your decks. Uploaded pictures can be fetched by anyone who knows their address, even without signing in; browsers and caches may keep them for up to a year. What your friends see through Baylee Central is set out in section 9a.1.

**6.4 Your assurance.** Upload only pictures and use only texts for which you hold the necessary rights. Pictures must not show other recognisable people who have not agreed, nor third-party trademarks or logos you are not allowed to use. Content, chat messages included, must not break the law or section 7.

**6.5 Deleting.** You delete decks and ban lists in the client. An uploaded picture is deleted once no account claims it any more, i.e. at the latest with your data on the gateway; I remove individual pictures on request. For chat see section 9a.4.

**6.6 Reporting unlawful content.** If you believe content is unlawful or breaches these terms, write to me at [E-MAIL] saying where it can be found and why. You report a chat message directly in the client (section 10.8). I review every notice promptly and carefully.

**6.7 Removal by me.** I may remove or block content that is unlawful or breaches section 7. Section 12 governs the procedure.

## 7. Conduct and fair play

**7.1 Respect.** Do not harass, threaten or insult anyone. Prohibited in particular are hate speech, discrimination, sexual or pornographic content, glorification of violence and unconstitutional symbols, whether in names, decks, ban lists, pictures, chat messages or reports.

**7.2 No deception.** Do not pretend to be another person, including me, other players, or representatives of Wizards of the Coast or Scryfall.

**7.3 Fair play.** Do not gain an unfair advantage. Prohibited in particular are spying on hidden information or having spectators or others pass it to you, falsifying moves, colluding against other players outside the teams the game provides, through chat too, or deliberately throwing games to falsify results or rankings. You may use modified clients under the AGPL, but not to gain such an advantage or to get around limits.

**7.4 Report bugs and loopholes, do not exploit them.** If you find a bug in the rules or the program, do not exploit it; report it through the report form or to [E-MAIL]. Please report security vulnerabilities only to me and publish them only after I have had a reasonable time to fix them. Anyone who finds and reports a vulnerability in good faith does not breach these terms, as long as they access no one else's data, harm no one and test no more than is needed to demonstrate it.

**7.5 Automation.** Play yourself. Bots, scripts and automated play are not allowed, except for the house AI and the AI seats the app itself offers (section 8); in ranked games you play exclusively yourself. [ASSUMPTION: Ranked games are for people only; docs/protocol.md says "two human seats".] It is also prohibited to create accounts automatically, to send messages automatically, and to fetch or redistribute card data, card texts, pictures or other data through the gateway in bulk.

**7.6 Technical integrity.** Do not attack the gateway. Prohibited in particular are overloading it, attacks on its availability, getting around limits, trying other people's credentials, accessing other people's accounts and relaying the gateway to third parties.

**7.7 Keys and tickets.** Seat tokens, socket tickets, chair tickets, sign-ins and game links carrying a seat token are meant only for their purpose. Do not pass them on; a chair ticket belongs only to your own seat bridge. Give table passwords only to people you invite to your table.

**7.8 No commercial use.** You must not use the gateway commercially, for example by selling accounts or seats.

**7.9 Consequences.** Breaches may lead to measures under section 12.

**7.10 Chat.** Sections 7.1 to 7.3 apply to chat as well. Also prohibited are spam, advertising and commercial offers, passing on other people's personal data without their consent, asking for passwords or other credentials, getting around a block, for example with another account, and knowingly false reports. Because I cannot read encrypted messages, there I mostly act only on a report (section 10.8).

## 8. House AI, AI seats and seat bridges

**8.1 House AI.** The gateway provides computer opponents. If you leave a running game or your connection drops, the house AI plays your seat once the reconnection window has passed; when you come back, you get it back. What the house AI plays in your place counts like your own move. In a ranked game, instead: if you leave it deliberately or your time runs out, you concede; if only the connection drops, you have a reconnection window, and if both connections drop, the game ends without a winner.

**8.2 AI seats.** The desktop client can use the seat bridge on your device to put a language model in a seat at your table. You choose the provider: for example an Anthropic interface or an OpenAI-compatible interface (such as DeepSeek or a local server), or an AI command-line tool signed in with your own subscription. For hosted AI seats, sections 8.10 to 8.12 apply instead of 8.3 to 8.8.

**8.3 Your contract with the provider.** For an AI seat you need your own account and, where applicable, your own key with the provider. I am not a party to that contract. You bear the costs and must comply with the provider's terms, including whether it allows automated play on your subscription.

**8.4 Keys.** Your provider keys stay on your device: in your operating system's credential store or in an environment variable, never in the settings file. They go only to the provider address they are stored for, and never to the gateway or to me. You are responsible for the security of your device.

**8.5 Costs and caps.** The client offers spending caps per game, day and month and a spend book on your device. They are aids, not a guarantee: they calculate with the prices the respective version knows and with estimates. Only your provider's bill is authoritative. If the seat bridge is killed outright, its reservation counts in full. I do not reimburse costs with your provider; liability is governed by section 15.

**8.6 What goes to the provider.** The seat bridge sends the model what your seat sees: the board, your hand, the list of your deck, the game log from your seat's point of view and the current question. Other players appear in it only as "P1", "P2" and so on, never by name, and no identifier of the game goes along. What is hidden from your seat is not told to the model either. If a profile of your seat bridge is set up that way, it also asks Scryfall about cards and their rulings, with the card name only. You initiate these transfers yourself under your own contract with the provider; the provider processes the data under its own responsibility, possibly outside the EU (for example in the USA or China).

**8.7 Your responsibility.** An AI seat plays on your behalf. Its name always begins with "LLM-", and the table and the record name you as the person it plays for. Section 7 applies to its conduct at the table as if you had played yourself.

**8.8 Chair tickets.** When you fill an AI seat, your client fetches a chair ticket. It is valid once and only briefly (120 seconds by default), and your client hands it directly to your seat bridge.

**8.9 What the record notes.** A game's record notes what answered each seat: a person, the house AI, a script, or a language model with provider, model identifier and reasoning level, for a hosted AI seat too. The seat bridge states this itself; it never contains a key, profile name or prompt text.

**8.10 Hosted AI seats.** The host of a table can choose a hosted AI seat: a language model that I run with a seat bridge on my server and through my own access to the provider, without you needing a key. For each model I decide whether every account may use it, only accounts I enable, or nobody. Which models I offer, and when, is up to me; there is no entitlement to them. I bear the costs.

**8.11 What goes to the provider with a hosted AI seat.** The seat bridge sends the model's provider only the state of the game as that seat sees it: the seating and teams, every seat's life and card counts, the battlefield, the stack and the public zones, the seat's own hand and its own deck with the rules texts of the cards, the moves of all seats since the last question, and the current question. All players appear in it only as "P1", "P2" and so on. Not transmitted are names, usernames and account identifiers, the game's identifier, players' IP addresses, language or location, other players' decks and hidden cards, and chat messages; the requests come from my server. These are therefore pseudonymous game data from which the provider cannot identify any player. The room shows the provider at the seat before anyone is ready; anyone who does not want this can leave the table beforehand. I initiate this transfer. The provider may be located anywhere, including outside the EU; whether and for how long it keeps the data is governed by its terms.

**8.12 Whom a hosted AI seat stands for.** A hosted AI seat plays for the host who ordered it: its name begins with "LLM-", and the table and the record name the host. If its model is busy or at its limit during the game, another model of the same provider that I have named for it plays the seat on; if the model is busy or limited when the game starts, the game may run on that other model from the start. Only if that is not possible either does the house AI play the seat on.

## 9. Records, replays, analysis and training

**9.1 What is recorded.** Every game on the gateway is recorded in full: the starting position with all decks, every move in order, all hands and libraries, so hidden cards too, and the details in section 8.9. The record names nobody; seats are numbered. Separately, I store which account sat in which seat, or on whose behalf a seat bridge played, and for the list of replays details such as the result and number of turns and, per seat, whether a person, the house AI or a model played.

**9.2 Hidden information.** Nobody at the table receives the record itself, and it cannot be retrieved through the game or the lobby. Apart from my own analysis (section 9.3), it goes only to a game process that computes a replay (section 9.7), and with all hands to Baylee Central if someone who sat at the game sends a report about it (section 10.2).

**9.3 What I use records for.** I use records internally to find and reproduce bugs, check the rules implementation, balance decks and cards, and develop and train the house AI. For this I always use the whole record; sharing under section 9.7 only governs what other players see.

**9.4 Pseudonymised, not anonymous.** As long as it is stored which account sat in which seat, a record is pseudonymised and not anonymous data. For balancing and training the house AI I use only an extract of the game content from which account identifiers and names have been technically removed; the link never enters balancing or training. If you delete your data on the gateway, your account identifier is deleted from that link, including where a seat bridge played for you; the game remains as a game between numbered seats. Details are in section 16.

**9.5 Retention.** I do not delete records; I keep them without a time limit, because bug finding, balancing and training need complete games. If you archive a game, it only leaves your list of replays; under "Archived" you still see it and can bring it back. The record, your fellow players' lists and your sharing stay. Your rights under section 16.9 remain unaffected.

**9.6 Games on your device.** If you play against the computer in the client without a gateway, the client keeps the last 20 such games on your device only. They leave it only if you expressly attach one to a single report.

**9.7 Replays.** You can watch again in the client every game on the gateway at which you sat, on another device too; the gateway computes it afresh on every request and does not store it. You can follow it from any seat's point of view. It shows your own seat whole, as you saw it. Another seat shows only what lay face up, unless it is shared for that game or belongs to the house AI or an AI seat (including one a player seated themselves). Then it shows what that seat saw: its hand, its deck list, what it was allowed to look at (for example when searching its library), its face-down cards, its own game log and its automatic answers; never clocks, chat or the rest of its library's order. It may also show the hands of further shared seats. What a shared seat was allowed to see of your cards during the game, for example because a card showed it your hand, can appear in its replay even if you do not share. Your sharing is on by default for games since 13 October 2026; whoever sat with you thus sees what you saw. For older games it is off. You untick or tick it per game in the dialog at the end of the game or later in the list, with effect for later replays. It stays when you delete your data on the gateway. [ASSUMPTION: The cut-off is the introduction of sharing with the version of 13.10.2026 (beta.7); `replay_share_cutoff` in the code, check the date before publishing.]

**9.8 Ranked games and tasks.** Baylee Central pairs you for a ranked game; for it, the gateway receives the pairing's identifier, the format and the two accounts. You submit a deck for it that can no longer be changed afterwards; the gateway checks it, forms a check value from it and has Baylee Central confirm that value. At the end the gateway reports the result to Baylee Central; Baylee Central keeps and publishes the rating under its own terms. If you play a task, the gateway reports to Baylee Central the result and the measured playing time of that attempt. Both kinds of game are recorded like any other (sections 9.1 to 9.5).

## 9a. Chat on the gateway

**9a.1 What there is.** On the gateway you write to your table, your team and, if they allow it, to individual players (whispers). In the lobby there is also the public lobby chat for everyone signed in. It is not encrypted (section 9a.2 does not apply to it): everyone connected, and I, can read it. The gateway keeps its last 50 messages in memory until it restarts. Friends, friend requests, blocks, groups, personal messages and your online status are offered by Baylee Central; its terms and privacy policy apply to them. With "Show friends which table I'm at" (on by default; Settings → Chat & privacy), your client tells Baylee Central, for as long as you sit at a table without a password, the gateway's address, the game's identifier, the table name and the number of seats taken, so that your friends can find you there. Who may whisper you you set in the same place.

**9a.2 End-to-end encryption.** The content of every message except the lobby chat's (text, card links, mentions) is encrypted on the sender's device for the recipients' devices and decrypted only there. The gateway receives and stores only encrypted content and has no key that opens it; I therefore can neither read nor hand over the content of your messages, except a message someone reports (section 10.8). This holds as long as the devices a conversation is encrypted for really belong to the people in it: the gateway keeps the directory of device keys for chat at the table, and a manipulated server could add a foreign device and read new messages. Your client therefore shows every new device of the people you talk to and of your own account, and with the safety number you check that you are writing to the right devices. A contact is "Verified" only if you mark it so yourself after comparing.

**9a.3 What I see nonetheless.** Only the content is encrypted. The gateway sees and stores each account's devices (random identifier, public keys, operating-system family, when set up and when last connected). For each message it stores, until delivery, which account sent it from which device to which conversation, when, and how large it is. In memory it holds who is connected right now and at which table, and invitations to tables. From this it can be seen who writes to whom and when. The friendships, requests and groups that were formed on the gateway before the move to Baylee Central have been deleted by the gateway, together with those groups' undelivered messages. Blocks from that time stay stored, because they still keep blocked players' table and team messages away from you, until you delete your data on the gateway or ask me to delete one of them. I write none of this to any log. Whoever may write to you sees the public keys of your devices with their operating-system family and set-up date.

**9a.4 Storage and deletion.** The gateway keeps a message, encrypted, only until every recipient device has fetched it, for at most 14 days. It stores no history; the history is kept encrypted on your devices, 30 days by default (adjustable under "Keep chat history on this device"). Delivered messages stay on the recipients' devices, even if you delete them on yours or delete your data on the gateway. Backups may contain undelivered messages, still encrypted, for at most 14 days (section 16.3).

**9a.5 Your devices.** The client keeps a device's chat keys encrypted on the device and in your encrypted vault at Baylee Central, which only you can open. Anyone who can use your device unlocked can read the history on it and write in your name. If you lose a device, remove it from another one under Settings → Chat & privacy and change your password at Baylee Central. That does not protect older messages: anyone who obtains a device's keys can read every message to that device of which they hold an encrypted copy. Messages are signed by the sending device; a recipient can prove that a message came from your device (section 10.8). An account can have up to 5 devices.

**9a.6 Who receives messages.** Table and team messages go to the accounts seated at that table or in that team when they are sent. Spectators, the house AI and AI seats, hosted ones included, receive none. Chat never enters game processes, records, replays, the training of the house AI or reports, except a reported message (section 10.8).

## 10. Bug reports and feedback

**10.1 Voluntary.** You send reports only when you press "Send" in the report form. The client sends crash reports only after you have agreed to that once.

**10.2 Contents.** A report always contains the kind of report, your text, the client's version and, if you write it at a table, the game's identifier; the gateway adds that game's record if you sat there. That record also contains the hidden cards of all other players, but no names. If your text refers to cards or players, the report notes which card and which seat is meant. Further details are added if their box is ticked when you send. At first all are ticked; what you untick stays unticked on that device, only the record of a game of your own is ticked anew for each report, unless you chose "Never offer". "Show what is sent" shows beforehand everything that goes along:

- System and hardware: operating system, processor count, graphics adapter, window size, language.
- The table as you see it: your view of the game, without anything hidden and without player names.
- Your game log: in English, with you as "You" and the others as "Player A", "Player B" and so on.
- Settings: language, display, volume, number of saved gateways, key bindings and automation; never your username, addresses or keys.
- Picture of the Baylee window: only the client's window as it was when the form opened, never the screen or other programs; it may show other players' names.
- The record of a game your client ran itself; for this one report only, and only after a further confirmation.

A report never contains a sign-in or seat key; the client checks this before sending.

**10.3 Pseudonym and recipient.** The gateway forwards the report with a pseudonym instead of your account to Baylee Central, where the administrators handle it. The pseudonym is a hash of your account identifier formed with a secret key; only the gateway has the key, so Baylee Central cannot read from it which account the report comes from. Your name, username, e-mail address, IP address and sign-in key are not passed on. The gateway's name, address and version go along with the report.

**10.4 Reports without signing in.** If you are not signed in to any gateway, the client sends the report to this gateway, which forwards it to Baylee Central without an account; beforehand the client shows you on a page of its own where the report goes and what it contains. It uses a random device identifier that is stored only on your device; only a hash of it formed with a secret key is passed on. The gateway counts your IP address in memory for one hour to limit abuse and does not store it.

**10.5 Basis, unticking and withdrawal.** Crash reports rely on your consent, which the client asks for once; you withdraw it in the form or in the settings under "Crash reports", from the next crash on. The other details in section 10.2 I process not on the basis of consent but to handle your report and find bugs (section 16.3); you can untick each of them before sending. I delete reports already sent on request.

**10.6 Reports stay with me; ideas.** Reports stay with me and the people named in section 16.6 and are not published. I may track the work on a bug publicly, for example in an issue in the project's repository on GitHub; such an issue contains only a neutral technical description written by an administrator, and never your report text, your pseudonym, player names, records, window pictures or any other information about people. I may implement ideas from reports in the project free of charge; there is no entitlement to implementation or payment.

**10.7 Retention.** A report at Baylee Central is deleted 90 days after it is done, at the latest after three years; a deleted one stays in the bin for another 30 days. Who handled it, when and how stays noted, never its content. Reported chat messages stay in the feedback service until an administrator deletes them; then they stay in the bin for another 30 days. Reports are not deleted automatically with your data on the gateway, because neither Baylee Central nor the feedback service knows your account behind the pseudonym.

**10.8 Reporting chat messages.** With "Report message" your client sends exactly that one message, with the key that opens only it, and your note to the gateway. The gateway checks the sender's signature and that the key opens the message to exactly that text, and forwards it as a report to the feedback service: text, note, kind of conversation, time sent, result of the check, for table and team messages the game's identifier, and a pseudonym under section 10.3 for you. Other messages and your history do not go along. By reporting, you disclose this one message to me. If the check held, the report names the sender by their public tag so that I can act on a breach; otherwise it does not name them at all, and it never names you so. Otherwise sections 10.6 and 10.7 apply.

**10.9 Contact form.** You can write to me with the form on the start page. The feedback service stores your message, your name and e-mail address only if you give them, the page's language and the time. The administrators read it and, if you gave an address, answer from their own mailbox. It is deleted 90 days after it is done, at the latest one year after it arrived; only who handled it and when is kept. Your IP address is counted in memory for one hour against abuse and not stored. For its visitors, the start page links its own short privacy notice.

**10.10 Anonymous count of offline games.** While the client itself runs a game against the computer, it sends the feedback service, if it knows one, a random number once a minute, so that I know how many people are playing offline right now. It is made anew for each game, held only in memory and says nothing about you or the game. The feedback service keeps it in memory until 150 seconds after the last signal and counts your IP address in memory for one hour against abuse, without storing it. The count is on by default; you switch it off under Settings → Privacy & Data with "Send anonymous usage count".

## 11. Availability, maintenance, changes and discontinuation of the service

**11.1 Maintenance.** I may interrupt the gateway at any time for maintenance, updates and restarts. I announce longer planned interruptions where I can.

**11.2 Changes to the service.** I may change, restrict or remove features where there is a valid reason, such as development during the beta, security, bug fixing, legal requirements or requirements of Wizards of the Coast or Scryfall. In doing so I take your interests into account.

**11.3 Discontinuation.** I may discontinue the gateway entirely. As a rule I announce this at least two weeks in advance in the client or on the gateway, so that you can export your decks. I may discontinue it without notice only for good cause, such as a security situation, an order by an authority or court, or a request by Wizards of the Coast.

**11.4 Blocked cards.** I may block individual cards on the gateway permanently or for a time, for example if they are implemented incorrectly; the client shows them with the reason. A deck with a blocked card cannot sit down at any table of the gateway; running games and offline play are not affected.

**11.5 Ending running games.** I may end a running game where there is a valid reason, such as an error that makes it unplayable, a breach of section 7 or urgent maintenance. Everyone at the table sees the reason; nobody wins or loses, and the record ends at that point.

## 12. Blocking, deletion and termination

**12.1 By you.** You can end the contract at any time by deleting your data on the gateway: in the settings (section 4.7) or, when a new version of these terms is shown to you, right there (section 13.4).

**12.2 Ordinary termination by me.** I may terminate the contract for an individual account with two weeks' notice if there is a valid reason. Section 11.3 governs discontinuation of the whole service.

**12.3 Measures for breaches.** If you breach these terms or the law, I may respond proportionately, in stages: a notice or warning, removal of content, ending a running game (section 11.5), a temporary block, a permanent block or deletion of the account on the gateway, and termination for good cause. I take into account the severity, frequency and fault of the breach and your legitimate interests.

**12.4 Procedure.** Before a block or deletion I warn you and give you the opportunity to respond, as far as this is possible and reasonable for me. Because the gateway knows no e-mail address of yours, I often cannot reach you until you contact me. A block always states its reason and, if it is temporary, its end; the client shows you both when you sign in. Write to [E-MAIL] to learn the reasons for a measure and to object to it; I review the objection and tell you the result.

**12.5 Immediate measures.** I may act without prior warning in the case of attacks on the gateway, obviously unlawful content, serious harassment, or where harm to others is imminent.

**12.6 Consequences.** When an account on the gateway is deleted, section 4.7 applies accordingly. I cannot restore deleted data. You must not evade a block with another account. While blocked you cannot sign in to the gateway, join a table or chat there; after the reconnection window the house AI plays your running games on for you. A block on this gateway does not affect your Baylee account at Baylee Central or on other gateways.

## 13. Changes to these terms

**13.1 Reasons.** I may change these terms where there is a valid reason, in particular changes in legislation or case law, new or changed features of the service, security requirements, closing loopholes and opportunities for abuse, or clarifying unclear provisions.

**13.2 Limits.** I do not change the main obligations of the contract unilaterally to your disadvantage. In particular, use stays free of charge, and I do not shift the balance between your rights and mine to your detriment.

**13.3 Procedure.** Every new version carries a date and begins with a short overview "What is new in this version". At your next sign-in the client shows you the new version and asks you to accept it. A change takes effect for you only once you accept it; your silence never counts as acceptance. Until you accept, you cannot play on the gateway: after signing in, the client stays at the terms. Purely editorial corrections that do not change the meaning (such as typos) I may make without a new acceptance.

**13.4 Declining.** You may decline a new version without incurring any cost or other disadvantage beyond losing access. If you do not want to accept the new version, you can delete your data on the gateway right there with "Delete my data on this gateway"; what is deleted is what section 4.7 lists, and your Baylee account remains. "Later (leave this gateway)" only leaves the gateway. If you cannot use the client, write to me at [E-MAIL] with your display name and tag; I will then delete your data on the gateway in the same way. As long as you neither delete them nor accept the new version, the previous version applies to your account; I may then terminate the contract under section 12.2.

## 14. Third-party rights, licences and notices

**14.1 Fan content.** Baylee is unofficial fan content. The notice under Wizards of the Coast's Fan Content Policy reads:

"baylee is unofficial Fan Content permitted under the Fan Content Policy. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC."

Baylee is not affiliated with Wizards of the Coast.

**14.2 Wizards' rights.** Card names, rules texts, card images and symbols belong to Wizards of the Coast. You acquire no rights in them; you may use them only within the game on the gateway. Wizards of the Coast may restrict the use of its intellectual property in fan content at any time; I would then have to adapt or discontinue the service (section 11).

**14.3 Scryfall.** Card data and images are provided by Scryfall (https://scryfall.com). You must not harvest or redistribute card data and images through the gateway.

**14.4 Source code.** Baylee's software is free software under the GNU Affero General Public License, version 3 (AGPL-3.0-only). Where the source code of the running version can be found is shown by the client on its start screen and by the gateway at https://gateway.baylee.app/source; currently that is https://github.com/AceVik/baylee.

**14.5 Further licences.** The client contains third-party works under their own licences, among them fonts under the SIL Open Font License 1.1, music recordings from the Versilian Community Sample Library, from Versilian Studios VSCO 2 Community Edition, from Osiris Piano and from FreePats, each under CC0, public-domain pieces of music, and deck lists from MTGJSON under the MIT licence. The details are in the NOTICE files shipped with the client.

## 15. Liability

**15.1 Unlimited liability.** I am liable without limitation for intent and gross negligence, for injury to life, body or health, under the German Product Liability Act, to the extent of any guarantee I have given, and where I have fraudulently concealed a defect.

**15.2 Slight negligence.** For slight negligence I am liable only if I breach an essential contractual obligation, that is, an obligation whose fulfilment makes the proper performance of the contract possible in the first place and on whose fulfilment you may regularly rely. Liability is then limited to the damage typical for the contract and foreseeable when it was concluded. Otherwise, liability for slight negligence is excluded.

**15.3 Free of charge.** Baylee is free. Where the law provides for milder liability for services rendered free of charge (compare §§ 521 and 599 of the German Civil Code, which limit liability to intent and gross negligence), that remains unaffected.

**15.4 Data and third-party services.** Section 3.2 describes the risks of the beta. I am not responsible for third-party services you use yourself, such as the providers of your AI seats, Scryfall or GitHub, or for their costs; section 15.1 remains unaffected.

**15.5 Your liability.** You are liable for your content and conduct under the statutory provisions.

## 16. Privacy information under Art. 13 GDPR

**16.1 Controller.** The controller for processing on the gateway and in the feedback service is [NAME], reachable at [E-MAIL]; the postal address is in the legal notice (section 2.1). I have not appointed a data protection officer; I am not required to. I am also the controller for Baylee Central; what happens to your data there is set out in its privacy policy (https://baylee.app/en/privacy).

**16.2 Accepting is not consent.** By accepting these terms you do not consent to any processing. This section only informs you. Consent exists only where it is expressly named: for crash reports (section 10.5).

**16.3 What I process, why and on what basis.**

- **Account:** a random identifier, the identifier of your Baylee account at Baylee Central and when the two were linked, your display name with tag (taken over from Baylee Central at every sign-in), language, time of creation, your chat settings (who may whisper you, whether friends see your table), and which hosted models I enabled you for, when and by whom. For a legacy account also its username and, if it was carried over from an even earlier version, possibly an e-mail address; the gateway no longer stores a password or password hash. Purpose: signing in and play. Basis: Art. 6(1)(b) GDPR. Duration: until you delete your data on the gateway.
- **Acceptance of these terms:** for a Baylee account, Baylee Central stores which version you accepted and when; at every sign-in the gateway learns only whether it is the one in force. Purpose: proof of acceptance. Basis: Art. 6(1)(f) GDPR. Duration: as long as your Baylee account exists.
- **Sign-ins:** a hash of the sign-in key, the account and the expiry (12 hours, extended on use). For a sign-in through Baylee Central the gateway holds in memory its identifier at Baylee Central and a hash until it ends, at most one hour without renewal, and the identifiers of ended sessions without an account for a little over an hour and a half. A ranked game has a sign-in of its own, limited to that game. Socket tickets (by default at most 45 seconds) and chair tickets (by default at most 120 seconds) I hold only as hashes in memory. I do not store IP addresses or the time of last sign-in. Basis: Art. 6(1)(b) GDPR.
- **Decks, ban lists and settings:** decks with name, format, description, card list, notes and version history, your own ban lists with name, format, cards and comments, and your settings (such as key bindings, automation, display, sound). Basis: Art. 6(1)(b) GDPR. Duration: until you delete them or your data on the gateway.
- **Sleeves and playmats:** an uploaded picture, re-encoded without the original file's metadata, and which account uploaded it; duration: as long as an account claims it. If you choose a sleeve or playmat from your inventory at Baylee Central for a game, Baylee Central permits the gateway to show it to the others at the table for that game and that seat; the gateway holds the permission and the details of the picture in memory only, until the game ends. Basis: Art. 6(1)(b) GDPR.
- **Tables and running games:** who sits where, deck name and a copy of the deck, table name, a hash of the table password, the table's ban list as a copy without its owner, whether spectators are allowed and who is watching (only the number is shown), in memory only; a waiting table for at most 2 hours, a finished game 1 hour after it ends. The game process receives the names of those seated, the decks and the moves, and ends with the game. Basis: Art. 6(1)(b) GDPR.
- **Ranked games and tasks:** what section 9.8 names; in addition, which game belongs to which pairing or attempt, until the result has reached Baylee Central. Purpose: playing out the game and reporting its result. Basis: Art. 6(1)(b) GDPR. Duration: the game like its record (section 9.5); the gateway deletes the link to the pairing or attempt as soon as Baylee Central has accepted the result.
- **Chat on the gateway:** what section 9a.3 lists; in memory only, who is connected right now and at which table, invitations to tables (10 minutes), and the last 50 messages of the lobby chat with their senders' display names and tags, until the gateway restarts. Purpose: section 9a. Basis: Art. 6(1)(b) GDPR. Duration: devices until you delete them or your data on the gateway; blocks from before Baylee Central as in section 9a.3; messages as in section 9a.4.
- **Records and replays:** as described in section 9, with the separate link from account to seat, your archivings and the sharing under section 9.7. Purpose: finding bugs, checking the rules, balancing, developing and training the house AI, replays. Basis: Art. 6(1)(f) GDPR, my legitimate interest being a game with few errors, good balance and good computer opponents, and for sharing your fellow players' interest in going over their games (you object by unticking); for replays, archiving, ranked games and tasks Art. 6(1)(b). Duration: without a time limit; the link and your archivings end when you delete your data on the gateway.
- **Hosted AI seats:** what section 8.11 lists, to the model's provider. Basis: for the host Art. 6(1)(b) GDPR, for the others at the table Art. 6(1)(f); my legitimate interest is to offer computer opponents with language models without anyone needing a key of their own.
- **Reports:** as described in section 10, reported chat messages included. Basis: for crash reports your consent (Art. 6(1)(a) GDPR), otherwise Art. 6(1)(b) and (f) GDPR (handling your report, fixing bugs, moderation); for your fellow players' game data in an attached record and for the sender of a reported message Art. 6(1)(f) GDPR. Duration: as in section 10.7.
- **Moderation:** on the account, the reason, start, end and administrator of a block until the next block, its lifting or the deletion of your data on the gateway. Every measure, an enabling under 8.10 included, is logged with the time, administrator, kind, identifier of the account, game or card and end of a block, never with a reason: in the server log for at most 14 days (see server logs), in the feedback service's audit log without a fixed limit. Basis: Art. 6(1)(b) and (f) GDPR (safe, fair operation; proof of my measures).
- **Contact form:** as described in section 10.9. Purpose: answering your enquiry. Basis: Art. 6(1)(f) GDPR, for questions about your account (b). Duration: 90 days after it is done, at most one year.
- **Anonymous count:** as described in section 10.10. Purpose: knowing how much is played offline. Basis: Art. 6(1)(f) GDPR. Duration: 150 seconds, the IP address one hour, both in memory only.
- **Abuse protection:** counters for wrong table passwords, reports, chair tickets, chat functions, replays, card images, the contact form, the start page's legal notice and the anonymous count, keyed by IP address or account depending on purpose; in memory only and only for the respective time window (between one minute and one day, depending on purpose). Basis: Art. 6(1)(f) GDPR; my legitimate interest is the security of accounts and the availability of the service.
- **Server logs:** operational messages such as game identifiers, seat numbers, counts and error messages. No log line names a username, display name, e-mail address, IP address, key, chat content or conversation; only the lines about administrators' measures name an account identifier. The web server in front writes no access log. Basis: Art. 6(1)(f) GDPR (operation, troubleshooting and fending off attacks). Duration: at most 14 days, in the server's system journal as in the log files the operating system also writes the same messages to. [OPEN: The log files have been limited to 14 days since 25.11.2026; older entries from before are gone from about 08.12.2026. The system journal still keeps older entries despite its "14 days" setting, because it deletes only whole files: before publishing, set `MaxFileSec=1day` there and delete the files of a former machine ID.]
- **Backups:** I back up the databases of the gateway and the feedback service regularly, so that the service can be restored after a failure. Each backup is encrypted on the server before it leaves it and is then stored in my storage on Google Drive (sections 16.6 and 16.8); only I hold the key. Undelivered chat messages stay in the backups for at most 14 days, everything else for at most 12 months. Deleted data may therefore remain in a backup up to these limits. Backups serve only to restore the service; if I restore one, I delete again what was deleted since it was made. Basis: Art. 6(1)(f) GDPR; my legitimate interest is that the service stays available and can be restored after a failure (Art. 32(1)(c) GDPR). [OPEN: Publish only once the backup runs this way (backup plan, its own thread); until then this paragraph, 9a.4, 16.6 and 16.8 are not true. If a second encrypted copy on a drive of my own is added, name it here.]
- **Statistics:** the gateway tells signed-in users how many players are online and how many tables are waiting or running, and reports counts to Baylee Central for its directory of gateways (players connected, players in games, open tables and games); it stores no identifiers for this and never names anyone. What the administrators see (16.6) is not stored in addition either.

**16.4 On your device.** The client stores on your device, among other things, your language, the last sign-in name used, your gateway list, your choices for reports, settings, a copy of which version of the terms you accepted at which gateway, decks saved offline, cached card texts and images, a crash file where applicable, records of your own offline games, a random device identifier for reports, the settings and spend book of your AI seats, and your chat keys and encrypted chat history (section 9a.5). In the web version these data are kept in your browser's storage; the gateway sets no cookies. Storing them is strictly necessary for the features you request (§ 25(2) no. 2 of the German TDDDG). These data leave your device only as described in these terms, and you can delete them at any time.

**16.5 Anonymised and pseudonymised.** Anonymised data can no longer be linked to a person; pseudonymised data only with additional information kept separately. I use data internally only in the following form:

- To store your decks, ban lists, settings and account I use your data as they are, solely for you.
- Records are pseudonymised as long as the link from account to seat exists (section 9.4). For balancing and training the house AI I use only an extract from which account identifiers and names have been technically removed.
- After your data on the gateway are deleted, a record no longer has any link to your account. I cannot entirely rule out that an unusual deck might allow conclusions in individual cases; I make no attempt to link records to people.
- Reports are pseudonymised (section 10.3). A reported message names its sender by their tag if the check held (section 10.8).

**16.6 No disclosure, with clear exceptions.** I do not sell data, do not use it for advertising, do no tracking and no profiling for advertising or marketing purposes, and make no automated decisions within the meaning of Art. 22 GDPR; the house AI and AI seats only make moves in the game. I disclose data only in these cases:

- **Hosting:** the gateway, game processes, feedback service and databases run on a rented server of netcup GmbH in Karlsruhe, Germany, which acts on my behalf and on my instructions; the seat bridges of hosted AI seats run there too, and the backups are made there. A data processing agreement with netcup is in place (Art. 28 GDPR).
- **Where backups are kept:** the encrypted backups (section 16.3) are kept in my storage on Google Drive; the provider is Google Ireland Limited, Gordon House, Barrow Street, Dublin 4, Ireland. Google does not receive the key and cannot read the content.
- **Baylee Central:** Baylee Central, which I also operate, confirms your sign-in and the version of these terms you accepted, and receives from the gateway reports (section 10), the results of ranked games and tasks (section 9.8), the confirmation of sleeves and playmats from your inventory, and counts for the directory of gateways.
- **Fellow players and other users:** they see what sections 6.3 and 9a describe.
- **Providers of hosted AI seats:** if a hosted AI seat plays at a table, the model's provider receives the pseudonymous game data under section 8.11. The room shows at the seat which provider that is.
- **Administrators:** only they see reports, contact messages and the management of the gateway: accounts with display name and tag, for legacy accounts the username, language, time of creation, whether an e-mail address is on file (never the address), blocks and enablings, when last active, the name, format and size of decks (not their cards), latest games with fellow players, and who is online where; to enable someone they search accounts by the start of their name. They do not see chat content, except reported messages and the public lobby chat.
- **Legal obligations:** to authorities or courts where I am required to. I cannot disclose the content of encrypted chat messages in doing so, because I cannot decrypt it, except reported ones (section 10.8).

Not disclosed by me but initiated by you are: the data your client exchanges directly with Baylee Central (its privacy policy applies to that); the data your own seat bridge sends to the provider of your AI seat and, where applicable, to Scryfall (section 8.6); and connections your client or browser makes directly, namely to Scryfall (Scryfall, LLC, USA) for card images, card texts and printings, for the desktop client at start-up and every six hours to GitHub to check for updates (can be switched off in the settings), and on the gateway's start page to GitHub for the list of releases. These providers see your IP address and the request, but neither your account nor a sign-in key.

**16.7 When others use an AI seat.** If another player's AI seat or a hosted AI seat sits at your table, its provider receives what that seat sees, which includes your openly played cards and moves, but not your name and no chat messages. You can recognise an AI seat by its name "LLM-…".

**16.8 Third countries.** I myself store personal data in only one way that lets it reach a country outside the EU and EEA: Google may keep the backups (section 16.3) in data centres outside the EU as well, for example in the USA. They are encrypted then, and only I hold the key. [ASSUMPTION: For the USA, the European Commission's adequacy decision of 10.07.2023 on the EU-US Data Privacy Framework, under which Google LLC is certified, applies; check whether it covers a private Google account, otherwise name the standard contractual clauses in Google's terms.] The providers of hosted AI seats may be located anywhere, including outside the EU; they receive only the pseudonymous game data under section 8.11, from which they cannot identify any player. For the connections you initiate, the data go directly from your device to the respective provider.

**16.9 Your rights.** You have the right of access (Art. 15 GDPR), rectification (Art. 16), erasure (Art. 17), restriction of processing (Art. 18) and data portability (Art. 20). You may withdraw consent at any time with effect for the future (Art. 7(3) GDPR). Write to [E-MAIL] and give your display name and tag; to prevent misuse, I may ask for proof that the account is yours. You also have the right to lodge a complaint with a data protection supervisory authority (Art. 77 GDPR), in particular in the member state of your residence. The authority responsible for me is the Landesbeauftragte für Datenschutz und Informationsfreiheit Nordrhein-Westfalen (LDI NRW), https://www.ldi.nrw.de.

**16.10 After your data on the gateway are deleted.** What remains: the records of games played, without a link to your account; your sharing under section 9.7 (your fellow players then see your seat as a former player); messages already delivered, on the recipients' devices; reports already sent, at Baylee Central, and reported messages in the feedback service (I delete them on request); audit log entries about moderation measures; and backups until they expire. Your Baylee account at Baylee Central remains until you delete it there.

**16.11 Obligation to provide data.** You are not obliged to give me any data. Without a Baylee account you cannot play on the gateway; playing offline against the computer works without any account.

**16.12 Your right to object.** Where I process data on the basis of Art. 6(1)(f) GDPR (section 16.3: for example records, reports, moderation, abuse protection, server logs, backups, the contact form and the anonymous count), you may object at any time on grounds relating to your particular situation (Art. 21 GDPR), including to the use of your records. I then no longer process the data unless I can demonstrate compelling legitimate grounds that override your interests, or the processing serves the establishment, exercise or defence of legal claims. An informal message to [E-MAIL] is enough.

## 17. Final provisions

**17.1 Governing law.** The law of the Federal Republic of Germany applies. If you are a consumer habitually resident in another country, you keep the protection of the mandatory consumer protection provisions of that country.

**17.2 Dispute resolution.** As a private, non-commercial operator I am not obliged and not willing to take part in dispute resolution proceedings before a consumer arbitration board.

**17.3 Invalid provisions.** If a provision of these terms is wholly or partly invalid, the rest of the contract remains valid. The invalid provision is replaced by the statutory provisions.

**17.4 Contact.** Send questions about these terms, notices and requests about your data to [NAME], [E-MAIL].
