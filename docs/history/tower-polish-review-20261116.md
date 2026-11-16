# Tower polish author review — 16 November 2026

Scope: the second Tower playtest's requested shop, draft, reward, table and
shared-editor changes after Gemini `d7111b6d0`, paired with Central `87d40e86`.
This is the implementing author's review after more than five correction passes,
not an independent audit or permission to merge/deploy. Evidence and unsuccessful
iterations are retained in `../proposals/tower-polish-20261116.md`.

## Boundaries reviewed

- Live transport authenticates the owner again for every request/push and on
  heartbeat. Private inventory never comes from a broadcast notification.
  Bounded subscriptions, immutable-publication size, account/socket limits and
  timeouts constrain memory/work. Credential refresh does not reset account
  budgets. Commands keep durable request identities and revision checks.
- Run state, prices, fusion, draft selection, refits and rewards remain authority
  decisions. UI animations describe confirmed revisions. Ambiguous replies are
  not treated as success and are retried with the same mutation identity.
- New generator/content/executor/AI versions preserve older replay semantics.
  Frozen prices, paid resale basis, first equipped fusion slot, double claims,
  frozen shop refill and private enemy potions have focused regressions.
- The scoped shared editor cannot write the normal library/autosave, accept late
  ordinary pool replies, add unowned nonbasics, exceed 40/5 or silently delete a
  moved card when its target is full. An acknowledgement cannot clean later edits.
- Exact-edition image translation reads only public metadata and earned table
  prints, is bound to one Tower game and never rewrites replay identity. The
  sorted runtime mapping is about 18 KiB; metadata fetching is offline by default,
  fixed-origin, bounded and redirect-free. No dependency or custom crypto was added.
- Item detail views recognize compiled identities rather than names. The table
  chooser validates current ownership and legal actions. Hidden potion data stays
  absent from opponent/spectator views and normal card links retain their policy.

## Findings corrected during review

1. Native ordering rows dropped token-source art. They now use the same public
   token-before-card precedence as stack rows and the shared item detail renderer.
2. A potion chooser lost its UI anchor when the inventory closed; a one-option
   sheet was also dismissed by keyboard handling. A visible item anchor and the
   standard arm/confirm path now survive until the action is answered. Native QA
   verifies the actual charge decrement and stack effect, not just menu state.
3. The editor's cold entry built the unrelated partner catalogue and retained an
   old library page. Shared entry now closes it, and single-commander pool rows
   reuse metadata without constructing partner pairs.
4. Mutation feedback and settlement income needed different deduplication rules.
   Adjacent-revision purchase/fusion animation and a separate per-round income
   latch handle replayed replies and skipped worker revisions without extra gold.
5. Item log names opened generic card previews. Bounded distinct portrait margins
   now provide shared item details; all textual names remain readable. More opaque
   log glass improves reading when the stack is directly behind it.

## Remaining release evidence

The construction/sampling tests and small duel pilots are not competitive balance
calibration for fifty decks and every item combination. Cross-service erasure and
least-privilege production provisioning retain the documented operator gates.
Central's canonical Linux WASM comparison remains blocked by Docker's read-only
overlay on this test host; its failed recheck created no container. Both PRs stay
draft for the owner's Fable review and these release checks. No real users,
production credentials, migrations or deployment were used in QA.
