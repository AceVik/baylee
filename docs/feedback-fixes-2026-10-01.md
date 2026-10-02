# Feedback pass, 1 October 2026

Source: the live feedback service, read on 1 October 2026. A report is closed
only after its requested behaviour is verified. Fixes below describe source
changes; deployment is recorded separately. No private report attachments are
committed.

## 01a0f912-f535-7700-a9cf-8b212a105736

Im Loginscreen ist beim Übergang noch unschön graue Balken von den Input Feldern zu sehen.

Die Musik ist nicht schön. Sie sollte überarbeitet werden. Mehr wie Hans Zimmer aber ohne gegen Urheberrecht zu verstoßen. Und epischer im Kampf. AUsserdem ist Win und Lose Musik ein und die selbe. WIn sollte sich deutlich heroischer und epischer anhören.

Status: open.

## 01a0f910-06e1-7205-af12-ffb6d7381179

Es sollte die Möglichkeit geben "Mit allen angreifen" zu machen, die wie in forge funktioniert. Demnach auch einen Button "Alle zurückziehen" wie in forge.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Bulk attack and withdraw controls now remain available on every creature page. The actual choice-handler tests cover 50 creatures across all pages, multiple defenders and reversible drafts. Native dev-control verification selected all eight legal attackers and withdrew all eight while ChooseAttackers remained pending; no declaration was sent.

## 01a0f90f-49a4-755b-b48a-2df982c95361

Die nZielpaginierung sollte schöner sein mit Preview und On board Highlighting beim Hover über dem Ziel Button. Und Filtern nach Spieler.

Status: resolved in the feedback service (59b64229; source/native verified; not deployed).

Target choices now have a separate controller-filter row (all players / each
seat with legal choices), bounded pages retaining original option indices,
and selection summaries that survive filtering. Hovering a target or attacker
button previews that precise card and highlights its battlefield representative,
including non-representative members of merged piles. Page changes clear the
old hover; navigation, filters and hovering never submit a selection.

Validation: all 1173 client-core and 1153 client tests pass (2 client tests
ignored); dev-control/all-targets clippy and native build pass. Native German
Giant Growth test: 14 creature targets, filter to 10 opponents, second page
shows choices 9–10, hover previews object 133 and enlarges its visible group
(object 125) from 36.2×39.3 to 38.4×41.7 logical pixels. Selecting it, then
filtering to own four creatures preserves the selection and clears the hover.
Angreiferknopf preview also verified with empty assignments and no error.
Local evidence: /private/tmp/baylee-target-preview-live.png,
/private/tmp/baylee-target-hover-after.json,
/private/tmp/baylee-target-filter-selection.json,
/private/tmp/baylee-attacker-preview-live.png. No attachments committed.

## 01a0f90c-6eac-7447-a8a4-e166e44bfdc0

Stack sollte einen Scrollbalken haben und kann länger sein.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

The stack now has a visible draggable scrollbar, a taller viewport, and a window-height cap that keeps it above the hand controls. Native verification used eight/nine Ondu Cleric triggers: wheel scrolling moved both entries and thumb; the final panel ended above the hand HUD. All 26 stack tests pass.

## 01a0f907-cd55-77f0-8204-64aaf8e8ade6

DIe Menge an Kreaturen sowie die Lebenspunkte, das sieht nicht mehr sauber aus.

Status: resolved in the feedback service (7e1397b9 and 15e5f332; source/native verified; not deployed).

The attached report shows life 25476 wrapping below the heart. The fixed
72-pixel life cell was too narrow. The attached row now reserves 108 pixels,
with tighter inter-cell gaps; heart and total are independent, non-wrapping
texts. Five-digit totals retain the ordinary type size; exceptionally large
signed totals fit within the cell without dropping digits. Tests measure the
bundled font advances for positive/negative values through both i32 extremes.
All 11 seat-bar tests, dev-control/all-targets clippy and the native build pass.
Native smoke check confirms heart, life, hand and active-turn label remain
aligned at ordinary totals (/private/tmp/baylee-life-fit-live.png). Large
values were verified by font-metric tests, not a native high-life game.
The seventh continuation completes the crowded-creature portion with earlier
scrolling, a two-thirds visible card face and a clearly visible scrollbar.

## 01a0f907-59fa-7186-852d-a58d7e6ce51e

Y und Z sind vertauscht. Ich drücke Y passiert Nichts, bei Z gehts. Und ich muss gerade 100+ Mal Z drücken, hier wäre es gut, wenn man neben dem Ja Button noch ein "Ja für alle" oder sowas hat.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Yes/no letter shortcuts and their rebinding use the active keyboard layout; movement retains physical positions. QWERTZ Y/Z, QWERTY, rebinding and physical-only harness input are covered. A one-shot Yes to all button captures consecutive identical abilities already on the stack, cancels at unrelated choices and never approves later triggers or payments. The 101-trigger regression passes. Native verification resolved nine Ondu Cleric triggers from different permanents with one click, emptied the stack and correctly changed life from 40 to 130.

## 01a0f892-21f7-708e-a008-eff9363f20f8

Spirit Water Revival gibt mir eine Art Emblem für unbegrenzte Kartenzahl auf der Hand. Das sollte irgendwie/wo sichtbar sein für alle.

Status: resolved in the feedback service (commit ba2069c6; not deployed).

The public SeatView now carries no_max_hand_size, using the same engine query as cleanup. Every seat shows a localized No maximum hand size badge and the hand count with /∞. The field defaults to false for older JSON payloads. A real kicked Spirit Water Revival regression verifies that the effect is visible to both seats after the spell is exiled; Reliquary Tower regression verifies that a source-bound limit disappears when the source leaves. Native German-client QA with both seats confirmed the badges and counts. Validation: 195 host tests (2 ignored), 34 view tests, the actual Revival integration test and retained-UI invalidation test pass; client/host/view clippy with dev-control and all targets passes.

## 01a0f88f-0fe8-748d-a68d-0473d94d62bd

Das Flagship Vessel ist ab 8+ Charge Countern eine Artefakt Kreatur, sie sollte durch die Kombi auf dem Feld alle Kreaturentypen haben, damit auch Ally sein und für Mana tapbar sein.

Status: resolved in the feedback service (source verified; not deployed).

No rules defect in the reported position: the attached view has nine charge counters, artifact + creature types, every creature subtype and the granted five-colour mana ability. The log records Inspirit being cast this same turn (turn 19); summoning_sick is true, so its tap-symbol mana ability is correctly unavailable. Station can still be activated by tapping another creature.

## 01a0f88b-65d1-71db-9dfc-541839052086

Inspirit, Flagship Vessel - es werden keine CHarge Counter angezeigt.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Charge counters have a persistent localized label outside the printing. Positioning prefers a free space beside the permanent and avoids covering neighbouring cards. Native verification activated Inspirit station by tapping Great Divide Guide: two charge counters appeared beside Inspirit. Six chosen-type/counter tests and client clippy pass.

## 01a0f889-1b73-7260-a154-b6b6d5397768

KI handelt masuchistisch.

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Check the actual eligible targets before spending a removal spell. The reported enemy board was hexproof (Padeem + Mycosynth Lattice), leaving only the AI’s Birds of Paradise targetable. Regression covers this position and same-name collateral damage; target selection also values the whole Pulse exchange. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0f87d-af22-75aa-b125-515fd3158570

Auto Manaauswähler wählt favorisiert Kreaturen als Mana, er sollte viel viel klüger werden. Ich denke er macht es einfach nach der Rheinfolge wie sie gespielt wurden rückwerts. Besser ist es, wenn er es intelligent macht, sich die Hand das Feld etc. anschaut und guckt das er zuerst Mana verbrät, das tatsächlich Länder & Mana Artefakte sind. Dabei auch schaut, welches Mana könnte noch gebraucht werden für das was auf der Hand liegt etc. DIr fällt sicherlich was Kluges ein.

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Shared mana planner preserves creatures before interchangeable lands/rocks and weights remaining colour supply against coloured costs in hand. Floating mana and avoiding sacrifice/life costs retain precedence; regression checks source ordering, colour needs, unavoidable creature taps and floating mana. Both human auto-mana and AI use this policy. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0f879-b734-7218-9f2f-9d00c8940059

DIe KI ist wieder masuchistisch.

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Reject Toxic Deluge before spending the card when no X gives a positive exchange after friendly losses and life cost. The report was a zero-life, zero-effect cast over only the AI’s own creatures. Regression covers declining a losing exchange, accepting a useful one, and choosing the smallest effective X. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0f868-26c6-70c2-bf23-dc46193e6431

KI Wirkt Zeitliche Überlegenheit für ihre vollen Manakosten. Bitte prüfen ob sie hätte auch Miracle nutzen können und ob Miracle überhaupt funktioniert.

Status: resolved in the feedback service (565cf940; not deployed).

Miracle now opens a mana-only payment window after choosing X and targets if
floating mana is insufficient. The completed cast is held separately from
mana-ability questions, and the full price is projected as owed. The AI counts
available sources when accepting Miracle and choosing its X, then pays through
the existing debt planner. An unpaid cast leaves its card in hand, clears X,
and cannot reopen the offer. Held choices participate in the replay snapshot.

Validation: all 4266 engine tests (2 ignored), 198 AI tests, 195 gamehost unit
tests (2 ignored) and all gamehost integration/example targets pass. Clippy
with all targets and engine/fuzz passes. Real engine regressions cover Lotus
Petal's sacrifice/color choice, Entreat the Dead's X=2 and two targets, short
payments and the formerly pinned Metamorphosis Fanatic limitation. The actual
AI/gamehost integration earns a Temporal Mastery offer with zero floating
mana, taps exactly two Islands, pays {1}{U}, exiles the resolved spell and
queues its extra turn. This is engine/host verification; no UI change or
native client run was required for this fix. The original screenshot alone
does not establish whether that particular card was the turn's first draw.

## 01a0f864-b11f-76cb-9633-813213b8d856

Metamorphosis Fanatic hat Solitude zurück geholt, aber solitude hat funktioniert als ob es für Evoke gecastet wurde. Eigentlich müsste Solitude auf dem Feld bleiben.

Status: resolved in the feedback service (source verified; not deployed).

Fixed: clear the alternative-cast flag on leaving the spell/permanent lifetime. The actual evoke → graveyard → Reanimate regression failed before and passes after the fix. All 4263 engine tests pass (2 ignored).

## 01a0f860-bf81-71f5-945b-ff76c41d7991

Traumbild ist als falsches Token rein gekommen. Es hat 1/1 mein Token (von dem es als einziges als Kopie reinkommen konnte) hat 2/2

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Copied token art now matches name, base power/toughness, colours and card types, and requires a unique registry match. Ordinary pumps do not substitute current P/T for base P/T; ambiguous/name-only matches no longer choose the first token. Native verification cast Phantasmal Image and copied a Maskwood Nexus token: both displayed the blue 2/2 Shapeshifter art, with matching projected base values, colour and types. Regression also distinguishes the 1/1 token and rejects colour/type mismatches.

## 01a0f85f-1611-7548-888c-3d5f8ec58883

Maskwood Nexus hat keinen schönen Abilities Dialog & Effekte sowie Equip Effekte von Artefakten sollen ähnlich wie das Ausspielen von Karten aus der Hatd wirkbar sein.

Status: resolved in the feedback service (58351153; source/native UI verified; not deployed).

A single costly ability now opens the ordinary ability sheet with its printed
sentence. Engine-checked unpaid hints let the client plan mana before an
activation, including equip. Confirmation replans against the current pool and
sources and rechecks the final engine offer. Native German UI: Nexus displayed
its localized cost and sentence, tapped exactly three Forests only after
confirmation and made a blue 2/2 Shapeshifter. Bonesplitter then tapped one more
Forest, stopped for target selection, and equipped Grizzly Bears as 4/2.
Six regressions cover the payment path and single-row sheet. The feedback API
confirmed `resolved`.

## 01a0f85d-8ed7-7585-a62c-edd5344a070f

KI spielt removal auf ihre eigene Karte?

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Vanishing Verse evaluates its monochromatic target requirement before casting. A colourless opposing Maskwood Nexus no longer makes the AI exile its own Charming Prince. Regression verifies decline with only a friendly matching target and acceptance once an enemy matches. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0eacb-7403-76ca-b380-7b9a1dc2eefd

die karte alter ego kommt mit x   1+   1+    marken rein   in höhe  von X    das im manabetrag gezahlt wurde

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in 4e96d9fb. Tests cast Altered Ego with X counters and decline copying (which must not add counters). Verified by the 4263-test engine run.

## 01a0e8fb-5910-706a-98b0-225bd12a0c3f

Permanents und Card Preview größer, sowie zentriert in der Battlefield Zeile.
Der Alt Text, größer ebenfalls, kann man schlecht lesen. Generell alle Texte etwas größer.

Status: open; preview and Alt-text size improved in the seventh continuation.
Battlefield permanent size and general HUD text remain.

## 01a0e8f9-fb25-7062-a1f3-1eaf35c75a00

Unlicensed Harase Tap Effekt nicht einsethzbar (Sagt Ossi, der Spieler)

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in f423882a/e1a97c88. The engine tests play the exile ability against one graveyard, check its count, and crew the vehicle. Verified by the 4263-test engine run.

## 01a0e8f3-5447-71c3-87c8-89d42a196b74

Food token auf dem Stack zeigt weder Bild vom Token noch die Ability.
Food token Fähigkeit nur nutzbar, wenn vorher Mana manuell getappt, das sollte auch automatisch möglich sein, wie bei den Handkarten, wenn genug Mana verfügbar ist. (Betrifft alles auf dem Feld, auch Equip)

Status: resolved in the feedback service (2091fa1e; source/native verified; not deployed).

Food now pays automatically and retains its token image and full ability
sentence after sacrifice, including in a freshly reconstructed client view.
See the sixth continuation for tests and native evidence.

## 01a0e8d7-d898-73f4-a201-6482913cfce6

Es sollte möglich sein den Schaden auf Blocker selbst zu verteilen. (Mit einem Auto Button, der es dann automatisch die restlichen Punkte verteilt)

Status: resolved in the feedback service (b3bbad7d; source/native UI verified; not deployed).

Added “Rest automatisch verteilen” to the combat-number prompt. It preserves
previous manual shares and finishes only this source’s remaining division,
using marked damage and deathtouch; unrelated decisions and subsequent turns
are not approved. Engine integration: Craw Wurm versus four Grizzly Bears,
first share manually 1, next shares automatically 2 and 2, final remainder 1.
Native German UI reproduces the same sequence with Lure forcing all four
blockers; one auto-button click kills only the middle two Bears, first/last
survive, no client error. QA: /private/tmp/baylee-combat-auto-before.png,
/private/tmp/baylee-combat-auto-after.png and -after.json (not committed).
Validation: 1171 client-core tests, 1150 client tests (2 ignored), real-engine
integration test and dev-control/all-targets clippy pass.

## 01a0e8c1-8999-717b-99ec-93f44f79ae22

bei der karte memory  konte ich keine karten anschauen und wählen

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in f372192a. Memory Deluge tests cast it for four and flash it back for seven, select cards and verify destinations. Verified by the 4263-test engine run.

## 01a0e8bd-1226-7585-91eb-e0288b763048

Ich kann beim oko nicht alle optionen wählen

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in 32709c71. Tests play Oko’s Food, Elk and control-exchange abilities. Verified by the 4263-test engine run.

## 01a0e8b0-28ab-72b3-ab48-59b825687add

Der Tisch und der Himmel sind seltsam verpixelt.

Status: open.


## Overnight continuation

The owner explicitly requested twelve hourly continuations on 1 October;
heartbeat `baylee-nachtarbeit` is active in this chat. Finish remaining feedback
first, then complete the earliest unfinished set and continue by historical
first-print release order. Read CLAUDE.md, cards/AGENTS.md and current set
progress before card work; update docs/llm-learnings.md after each card batch.
Do not interpret the older roadmap’s retired model lanes as current tooling.

Local report attachments and the authenticated SSH helper are under
/private/tmp/baylee-feedback-*. They contain private report data and must not
be committed. Source fixes have not been deployed.

Set follow-up evidence: `docs/llm-learnings.md` sections “Limited Edition
Alpha, reader first” and “Alpha test backfill” record remaining Partials.
The current files for Balance, Gloom, Mana Flare and Black Vise still have
`Coverage::Partial` and explicit unsupported sentences. Alpha must be finished
before advancing; the existing cast/destination smoke tests are not full
rules coverage.

## UI validation milestone

The client/core/AI suites pass (1148 client tests, 2 ignored; 1169 core; 197 AI). Subsequent stack-height changes pass all 26 stack tests; counter positioning passes all 6 chosen-type tests. Clippy with dev-control and all client targets passes. Native screenshots and state snapshots remain in /private/tmp/baylee-*-live*.png/json; these are local QA evidence, not distributable assets.

## Next continuation: remaining investigations

20 of the original 25 open reports are resolved. Five remain open; finish them
before card batches. Commits: 8084106f (evoke lifetime), b4355e7f (AI and mana),
3f0ab837 (token/UI). No push or deployment has happened. The native test clients
started for this pass were stopped; no live test process needs preserving.

- Miracle: completed in 565cf940 and closed in the fourth hourly continuation;
  the engine payment window and AI source planning are verified above.
- Maximum hand size: completed in ba2069c6 and closed in the first hourly
  continuation; do not repeat this work.
- Food/activation: ability stack art depends on the still-present source;
  sacrificed Food has disappeared. Token abilities also lack the card text
  reference used by the stack sentence. Preserve truthful ability presentation
  across the sacrifice. Automatic mana for activated abilities/equip is
  implemented in 58351153; `legal.abilities` still means payable from floating
  mana, with separate `unpaid_abilities` planning hints. Verify Food itself
  alongside its missing presentation before closing that report.
- Pixel report: report build c3d5aa55 predates 2aae4a59's shared integer noise
  hash fix. Native Metal rendering and shader tests pass, but the reported
  Windows/Vulkan setup has not been reproduced; leave this distinction clear.
- Other remaining work: creature-density readability, larger centered cards/text,
  login-field transition and more distinct original battle/victory music.

## First hourly continuation

The hand-limit report is resolved in the feedback service, verified in source
and in the native client (ba2069c6). QA image: /private/tmp/baylee-hand-limit-live.png (not committed). The existing public effect is displayed; no new emblem object or rules exception is invented. The test client was stopped after verification. No push or deployment.

Next useful UI task: auto-distribute remaining combat damage. The engine already asks NumberPrompt::CombatDamage per recipient and assigns the last recipient the remainder (`engine/banding.rs`). The AI already computes a lethal share (`ai/policy.rs::combat_share`); the human prompt still only offers manual number confirmation. Any client-side auto action must stay limited to the explicitly approved division and stop at unrelated decisions.

## Second hourly continuation

Completed and verified the remaining-combat-damage button described above.
Client suites initially hit sandbox restrictions in eight local TCP tests;
rerunning with local port access passes all 1150 tests. Native test client
stopped after verification. No push or deployment.

The combat report was closed via the feedback API, which returned `resolved`.

The same continuation also fixes life-number wrapping (report 01a0f907-cd55),
with font-metric regression coverage and a native ordinary-total smoke check.
That report stays open for creature-density work. No test client is left running.

## Third hourly continuation

Target filtering and target/attacker previews are verified above. The native
test client was stopped. No push or deployment.

Crowded-board investigation: report 01a0f907-cd55 has 179 permanents, including
130 Allies split by actual counters, haste and summoning sickness. Eight Allies
are separately targeted by stack items. These differences must remain visible;
merging all same-name creatures would hide relevant state. Existing row
scrolling already follows a hovered group member. The creature-density portion
and the larger-card/text report still need a measured layout change.

The feedback API confirmed the target-pagination report as `resolved`.

## Fourth hourly continuation

The Miracle report is fixed, tested, committed (565cf940), and confirmed
`resolved` by the feedback API. No new reports appeared in the refreshed list.
Six of the original reports remain open. No push or deployment; no native
test client was started. Local test logs: /private/tmp/baylee-miracle-tests.log
and /private/tmp/baylee-miracle-clippy.log.

Next: activation/equip automatic mana and the Maskwood/Food reports, then the
remaining visual/audio feedback. Alpha remains the earliest incomplete set;
finish feedback before returning to its four recorded Partial cards.

## Fifth hourly continuation

Implemented automatic mana for activated battlefield abilities, including
Maskwood Nexus and equip. The engine exposes separate unpaid planning hints
after non-mana costs, targets, timing and locks pass. Only actual paid offers
authorize actions. The client reserves the source, arms a mana plan with the
ability's readable sheet, replans on confirmation, and hands later target or
cost questions back to the player. Five real-engine regressions cover Nexus
(including its blue 2/2 token), equip, manual pre-tapping, Karn's activation
lock, missing targets and insufficient sources. A sixth regression checks
that a single armed ability actually produces its sheet and printed words;
native QA caught an old renderer guard that suppressed lists shorter than two.

Validation: 1159 client, 1173 client-core, 4266 engine, 198 AI,
195 host and 146 seat unit tests pass, along with the ability-sheet, mana,
network and host integration suites. The broad run found an existing failure
in `baylee-seat --test selfplay how_often_the_mind_is_woken`: p90 249 wakes
exceeds 150. A separately rebuilt archive of HEAD (8f664d77) produces the
identical four seat-game counts (median 54, p90 249); this activation change
does not cause it. Keep it on the testing backlog rather than loosening the
threshold silently. Logs: /private/tmp/baylee-activation-full-tests.log and
/private/tmp/baylee-activation-baseline-selfplay.log.

Milestone 58351153 passed native verification at 1440×900 logical pixels and
clippy with `dev-control` and all targets for client and seat. QA image:
/private/tmp/baylee-activation-nexus-dialog.png; final view:
/private/tmp/baylee-activation-live-final.json. Nexus and equip both show the
localized ability text, spend exactly three and one mana respectively, and
retain explicit confirmation/target selection. The feedback API confirmed the
Maskwood/equip report as `resolved`. Twenty reports are now resolved; five
remain. The test client was stopped. No push or deployment.

Next: Food's missing stack art and text. `push_ability_to_stack` captures an
ability list but loses the source token's presentation after sacrifice;
`gamehost::view::stack_item` only exports printed-card rules/text, and
`client-core::board` gets ability art from `view.object(source)`, which is then
absent. Preserve source token identity and the ability index across this
lifetime, without marking the ability object itself as a battlefield token.
The token definition has no oracle-text field; use trustworthy existing token
text/presentation infrastructure and verify the sacrificed Food in a native
game. Its report stays open until both payment and stack display are proven.


## Sixth hourly continuation

Refreshed the feedback list: no new open reports. Food stack presentation is
implemented and verified by the full affected suites and native QA. `AbilityList` now carries token provenance through
cost payment, copies and look-back. `GameObject::own_origin` packs either the
card face or token identity into the existing four-byte slot, preserving the
312-byte object footprint. Host/view/board carry `TokenAbility` independently
of the vanished source and of the ability object's own token status. Food's
verified Scryfall Oracle sentence is shared by its ability sheet and stack.

Focused tests create Food with Oko, automatically pay two mana, verify the
source has ceased to exist while the stack keeps its token art and sentence,
reconstruct a fresh client view, and resolve for three life. A renderer test
checks the full sentence and rejects an unknown ability index. Card/token
origin round-trips and the footprint test cover the compact representation.
Validation: 1161 client, 1173 client-core, 4267 engine, 195 host, 198 AI,
129 cards and 35 view unit tests pass, plus the affected integration suites.
The additive optional wire field retains version 44; its shape fingerprint
and old-payload compatibility test are updated. Clippy passes for client,
host, engine and view with dev-control/fuzz and all targets. GameObject
remains 312 bytes. Logs: /private/tmp/baylee-food-full-tests.log,
/private/tmp/baylee-food-view-tests.log and
/private/tmp/baylee-food-clippy-final.log.

Native QA at 1440×900 logical pixels: create Food with Oko, activate with an
empty mana pool, confirm the plan, observe exactly two tapped Forests and a
vanished Food source. With Prodigal Sorcerer available to retain a reaction
window, the stack visibly shows the Food card image and full verified Oracle
sentence. Resolving increases life from 40 to 43 with no remaining mana or
error. Evidence: /private/tmp/baylee-food-stack.png,
/private/tmp/baylee-food-live-stack.json and
/private/tmp/baylee-food-live-final.json. The first run auto-resolved normally;
a second fixture named an unavailable card and was corrected before the
successful visual run. Its crash report was not sent. Test clients stopped.
No push or deployment.


Milestone 2091fa1e is committed. The feedback API confirmed the Food report
as `resolved`: 21 of the original 25 reports are resolved, four remain open.
Next: dense creature layout and larger readable permanents/preview, followed
by login transition/music and the platform-specific sky/table report. The
known baseline seat selfplay wake-count failure remains on the testing
backlog. Alpha's four Partial cards follow the remaining feedback work.


## Seventh hourly continuation

Refreshed the service: four open reports, no new reports. The dense-creature
row now keeps two thirds of each upright card visible, instead of a third,
and scrolls sooner. Distinct counters, sickness, targets and identities are
preserved; no additional cards are merged. Wide-duel row scrollbars use their
available seam instead of a subpixel hairline. A 130-card regression checks
individual reachability and the larger visible portion, alongside the
existing geometry, badge, plate, keyboard and scroll tests.

Preview/Alt-text portion of 01a0e8fb: base width 308 → 384 logical pixels;
constructed face name/type/body caps 26/19/18 pixels and body floor 14 instead
of 10. Long text scrolls rather than shrinking further. Window fitting and
aspect ratio remain enforced, including the larger scale settings and phone
viewports. Battlefield card size and general HUD type still need work, so
that report remains open.

Validation: all 1161 client and 1174 core unit tests, 16 headless duel,
33 mana/ability-sheet, 13 network-host and 7 play-caddy tests pass (three
pre-existing ignored tests). Clippy with dev-control/all-targets passes.
Local logs: /private/tmp/baylee-readability-tests.log and
/private/tmp/baylee-readability-clippy.log. The first non-escalated run could
not bind eight loopback test servers; the permitted rerun passed. The linker
reported its existing oversized unwind-section warning.

Native 1440×900 QA: 45 distinct creatures show 1–40 initially, then 6–45
after horizontal scrolling, without changing engine state. Oko's larger
German Alt-text is readable and scrolling reaches the complete last ability;
the full artwork preview retains its artist/copyright line. Evidence:
/private/tmp/baylee-readability-crowded.png,
/private/tmp/baylee-readability-row-start.json,
/private/tmp/baylee-readability-row-end.json,
/private/tmp/baylee-readability-preview.png,
/private/tmp/baylee-readability-preview-bottom.png,
/private/tmp/baylee-readability-art-preview.png.
The final wider scrollbar is visibly distinct and clear of both adjacent
rows (/private/tmp/baylee-readability-scrollbar.png). All four rowbar tests
and final dev-control/all-targets clippy pass after that last change. Test
clients stopped. No push or deployment.


Milestone 7e1397b9 is committed and the service confirmed the creature/life
report as `resolved`. Twenty-two of the original 25 reports are resolved;
three remain: permanent/general-HUD sizing (preview/Alt text now improved),
login transition/music, and the unverified Windows sky/table rendering.
Next continuation should finish permanent/general-HUD sizing. The native
1440×900 snapshot still shows battlefield cards about 39–46 logical pixels
wide; the preview is now 384. Inspect table framing and available lane space,
keeping centered packing, piles, badges, combat steps and 3–8 seat geometry
consistent. Do not claim the larger battlefield cards are already done.
Alpha work follows the remaining feedback. No push or deployment.


## Eighth continuation — permanent and general text sizing

Completed the remaining layout portion of 01a0e8fb. Wide desktop duels
use the playmat borders plus a small gutter for framing, recovering space
from the decorative rail. Native 1440×900 measurements on the same fixture:
Oko 39.9 → 42.3 px wide, Forest 40.6 → 43.2, Alaborn Grenadier 45.5 → 48.7
(projected bounds); centered nonoverflow rows and scrollable crowded rows
remain intact. Small windows and 3–8-seat framing retain their prior margin.

Shared general text scales increased ten percent. Expanded the game menu
and hand-tool reservations and adjusted stack truncation budgets. Three
initial regression failures exposed these tight bounds and an obsolete
life-total nominal-size assertion; final tests check full digits still fit
and five-digit life draws at least as large as before. The user explicitly
authorized a font replacement if useful. Current Alegreya Sans/Faustina
remain suitable after this pass; their existing OFL licences are recorded.

Validation: 1162 client unit tests, 16 duel, 33 mana/ability, 13 network
and one binary test pass; two pre-existing ignored tests. All-targets
dev-control clippy passes. Existing camera/seatbar/badge geometry tests
cover 2–8 seats and desktop/phone viewports. Native screenshots show the
full 45-creature fixture, centered Oko/lands, larger readable German ability
sheet and game menu, full copyright line and unclipped HUD controls.
Evidence: /private/tmp/baylee-framing-{row.json,crowded.png,abilities.png,menu.png},
/private/tmp/baylee-framing-tests-final.log and baylee-framing-clippy.log.
No push or deployment. Login/music and Windows Vulkan visual verification
remain; Alpha follows those feedback items.


Milestone 8a5cddda committed. The service confirmed 01a0e8fb as resolved:
23 of the original 25 reports are resolved. Two remain: login/music and
Windows sky/table. The current follow-up reproduces the login bars as
primary button shader surfaces that ignore panel alpha, not text input data.


### Login/music milestone

Reproduced the solid transition bar natively, paused at 5% clock speed.
Its source was the primary-button shader's hardcoded alpha 1.0, combined
with a shared material outside the existing text/frame fade. Shader opacity
now follows the panel through a per-surface material cloned once on first
fade. A regression checks both panels, unchanged shared material, bounded
clone count and reversing the passage. Before/after/reverse screenshots:
/private/tmp/baylee-login-{before-late,after-late,after-reverse}.png. The opaque
bar is gone in the same intermediate phase.

Battle now foregrounds horns with quieter piano accompaniment and an
ostinato/timpani pulse at low activity. Victory has its own original D–G–A–D
brass fanfare, strings and percussion; defeat uses quiet piano/cello with
no brass or percussion. Outcome scheduling no longer reuses the sanctuary
piano pattern. Existing CC0 recordings are unchanged; publisher licence
rechecked directly. The 150-second runtime demo rendered successfully to
/private/tmp/baylee-score-review.wav. This is an audition artifact, not a
claim of subjective approval of the new music.

All six music tests pass, including continuous transport/cadence completion
and sampled output bounds. Measured 12-second RMS: battle 0.101, victory
0.082, defeat 0.024; maximum peak across moods 0.389 and sample jump 0.064.
The outcome contrast regression requires victory RMS > 1.5× defeat.
All 1163 client unit tests, one binary, 16 duel, 33 mana/ability and 13
network tests pass (two existing ignored tests). Client/core all-targets
clippy passes. Logs: /private/tmp/baylee-login-score-tests-final.log,
baylee-login-score-clippy.log, baylee-score-tests.log, baylee-score-render.log.
No push or deployment.

New user steering: after feedback, review gameplay and the lobby for UX/UI
inconsistencies; document findings, implement improvements with multiple
screenshot iterations. Specifically shorten chosen-type labels and use
count plus icon for charge counters. This review precedes Alpha card work.
Font replacement is explicitly permitted when readability, compactness and
licensing justify it. No parallel-agent authorization was added.


The service confirmed login/music report 01a0f912 resolved after milestone
fbe381ac: 24/25 original reports resolved. Windows renderer verification is
still open. The requested UX pass has begun; see docs/ux-review-2026-10-02.md
for the native before/after screenshots, first implemented improvements and
remaining iteration work.

## 2026-10-02 — requested UX follow-up, waiting room

Continued the user-requested multi-iteration client audit in
`docs/ux-review-2026-10-02.md`. At 1280×800 the room now groups seat identity,
readiness and deck controls compactly, expands starting cards on demand and
keeps Start/Leave visible above its scroller. Count, draft and focus behavior
are tested; screenshots cover before, after, expanded and scrolled states.
Offline room guidance now describes AI/deck setup. Client unit and integration
suites and clippy pass; no feedback report was closed for this separate UX work.

## 2026-10-02 — gameplay annotation verification

The next screenshot iteration reproduced a charge-counter explanation hidden
behind its own hover preview. Annotation placement now avoids full preview
rectangles, card prints and other annotations, uses measured text dimensions
and respects window bounds. Four core geometry regressions, the full client
suite and clippy pass. Native before/after screenshots and the room-driven
three-counter fixture are recorded in `docs/ux-review-2026-10-02.md`.
Personal deck summaries now show copy totals rather than stored line counts;
older gateway payloads retain a correctly labelled row-count fallback.

The final feedback-service read shows **50 resolved, 1 new** across the whole
log. The sole open entry remains `01a0e8b0-28ab-72b3-ab48-59b825687add`, the
Windows Vulkan report awaiting affected-renderer verification. No new report
appeared during this UX pass. The login form hierarchy remains an audit item;
historical Alpha completion has not yet resumed after the user's UX priority.
