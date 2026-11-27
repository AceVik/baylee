-- The runtime role's privileges, exactly those its startup check requires
-- (docs/operations.md §"Provisioning"). Idempotent: the deploy hook
-- (ops/deploy-hook) runs it after every migration. A migration that adds a
-- table adds its grant here. The hook applies it as central_migrator, never
-- as a superuser, and takes SQL only: it refuses the file if a backslash is
-- anywhere in it, here in a comment too.
GRANT USAGE ON SCHEMA central_identity TO central_runtime;
GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions,
  central_identity.private_objects TO central_runtime;
GRANT SELECT, INSERT ON central_identity.migration_archives TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
-- Administrators approve, enable and disable gateways on the website
-- (docs/administration.md §"Gateways"); an approval is never deleted.
REVOKE INSERT ON central_identity.gateway_approvals FROM central_runtime;
GRANT SELECT, INSERT (gateway_id, enabled), UPDATE (enabled, ranked_verified) ON central_identity.gateway_approvals TO central_runtime;
-- Gateway tokens (docs/gateway-registry.md §"Gateway tokens"): stored by
-- digest; afterwards only their revocation and last use change.
GRANT SELECT, INSERT, UPDATE (revoked_at, last_used_at) ON central_identity.gateway_tokens TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins TO central_runtime;
-- Session tokens (docs/gateway-registry.md §"Session tokens"): a session as each gateway
-- knows it, and each gateway's revocations, which the runtime can add and age out but
-- never rewrite.
GRANT SELECT, INSERT, UPDATE ON central_identity.gateway_streams TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_sids, central_identity.gateway_revocations TO central_runtime;
GRANT UPDATE (ranked_proof_until) ON central_identity.gateway_sids TO central_runtime;
GRANT SELECT, INSERT ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
  central_identity.terms_acceptances TO central_runtime;
-- Registration open or closed: read by every registration, changed by an
-- administrator (docs/registration-abuse.md); the row itself stays.
GRANT SELECT, UPDATE (open, changed_at, changed_by) ON central_identity.registration TO central_runtime;
-- Administrators' decisions are the runtime's (docs/administration.md): it
-- grants and removes roles, and writes the audit, which it can neither rewrite
-- nor delete; the operator's sweep deletes events after three years.
GRANT SELECT, INSERT, DELETE ON central_identity.account_roles TO central_runtime;
GRANT SELECT, INSERT ON central_identity.audit_events TO central_runtime;
-- Account-owned contacts: immutable blocks and bounded per-account budgets.
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.social_contacts TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.social_blocks TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.social_budgets TO central_runtime;
-- Friend invites: runtime creates single-use invites and updates reservation, redemption or revocation.
GRANT SELECT, INSERT ON central_identity.friend_invites TO central_runtime;
GRANT UPDATE (pending_account_id, pending_until, redeemed_by, redeemed_at, revoked_at)
  ON central_identity.friend_invites TO central_runtime;
-- Encrypted chat: immutable public keys, signed ciphertext and delivery wraps.
GRANT SELECT, INSERT, DELETE ON central_identity.chat_devices, central_identity.chat_messages,
  central_identity.chat_deliveries TO central_runtime;
GRANT USAGE ON SEQUENCE central_identity.chat_deliveries_seq_seq TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.chat_groups, central_identity.chat_members TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.chat_group_pictures TO central_runtime;
GRANT UPDATE (key, avif, png, sequence, sha256, side, set_at) ON central_identity.chat_group_pictures TO central_runtime;
-- Pairings (Baylee's deep links): the runtime starts, claims, approves and
-- redeems them, and deletes each when it is used or over.
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.pairings TO central_runtime;
-- Reports (docs/reports.md): the runtime files them and their parts once,
-- triages them by column, and deletes them (the trash's purge and the
-- sweep); it never changes a report's words, a part or the history.
GRANT SELECT, INSERT, DELETE ON central_identity.reports TO central_runtime;
GRANT UPDATE (status, issue_number, assignee, priority, labels, duplicate_of, revision,
  done_at, deleted_at, updated_at) ON central_identity.reports TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.report_parts, central_identity.report_notes,
  central_identity.report_seen TO central_runtime;
GRANT SELECT, INSERT ON central_identity.report_audit TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.report_views TO central_runtime;
GRANT UPDATE (query) ON central_identity.report_views TO central_runtime;
-- The legacy store (docs/operations.md §"The legacy store"): its own role,
-- which reads its accounts, deletes a row when its move closes and counts the
-- lockout, and nothing else; the runtime role gets nothing in the schema.
-- The import, the deadline and the sweep stay the migrator's. Only where the
-- operator made the role: a deployment without it serves no legacy sign-in,
-- and a service configured for one refuses to start (its role check).
DO $legacy$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'central_legacy') THEN
    GRANT USAGE ON SCHEMA central_legacy TO central_legacy;
    GRANT SELECT, DELETE ON central_legacy.accounts TO central_legacy;
    GRANT UPDATE (failures, locked_until) ON central_legacy.accounts TO central_legacy;
    GRANT SELECT ON central_legacy.settings TO central_legacy;
  END IF;
END $legacy$;

-- Cookie-owned push preferences, bounded subscriptions and coalesced delivery hints.
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.notification_preferences, central_identity.push_subscriptions, central_identity.push_jobs TO central_runtime;
GRANT SELECT, INSERT ON central_identity.push_releases TO central_runtime;
GRANT EXECUTE ON FUNCTION central_identity.queue_push(uuid,smallint), central_identity.push_event() TO central_runtime;
-- An account's optional e-mail address (docs/email.md): the runtime names,
-- confirms, replaces and removes it, and keeps on its row what a reset code
-- by mail needs. The operator's sweep deletes addresses whose code is over.
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.account_emails,
  central_identity.account_email_pending TO central_runtime;
-- Profile pictures (docs/profile-pictures.md): the runtime sets, replaces
-- and deletes an account's picture (never its account), files picture
-- reports and decides them by column but never deletes one, keeps the
-- person's notice and block, and counts uploads. The operator's sweep
-- deletes decided reports a year after their decision.
GRANT SELECT, INSERT, DELETE ON central_identity.pictures TO central_runtime;
GRANT UPDATE (key, avif, png, sequence, sha256, side, set_at) ON central_identity.pictures TO central_runtime;
GRANT SELECT, INSERT ON central_identity.picture_reports TO central_runtime;
GRANT UPDATE (state, decided_at, decided_by) ON central_identity.picture_reports TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.picture_notices TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.picture_budgets TO central_runtime;

-- Global daily/weekly tasks: frozen publications may only be withdrawn.
GRANT SELECT, INSERT ON central_identity.challenges TO central_runtime;
GRANT SELECT, INSERT ON central_identity.tower_seasons, central_identity.tower_nodes TO central_runtime;
GRANT UPDATE (published, withdrawn) ON central_identity.tower_seasons TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.tower_actions TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.tower_participation,
    central_identity.tower_runs, central_identity.tower_scores,
    central_identity.tower_accomplishments TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.tower_access TO central_runtime;
GRANT SELECT ON central_identity.tower_award_issuers TO central_runtime;
GRANT UPDATE (last_used_at) ON central_identity.tower_award_issuers TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.tower_award_runs,
    central_identity.tower_award_progress TO central_runtime;
GRANT SELECT, INSERT ON central_identity.tower_award_receipts TO central_runtime;
GRANT UPDATE (withdrawn) ON central_identity.challenges TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.challenge_participation,
    central_identity.challenge_attempts, central_identity.challenge_completions TO central_runtime;

-- Donations (docs/operations.md §"Donations", migration 000032): the runtime
-- adds a row once and converts one that came without a rate, and reads the
-- sums and the day the count began. It never changes an amount and never
-- deletes a row or writes a sum: the operator's sweep folds rows older than
-- 40 days into the sums under the migrator's login.
GRANT SELECT, INSERT ON central_identity.donations TO central_runtime;
GRANT UPDATE (euro_cents) ON central_identity.donations TO central_runtime;
GRANT SELECT ON central_identity.donation_sums, central_identity.donation_count TO central_runtime;

-- Ranked authority. Only the operator publishes seasons. The host offer and the
-- two occupied seats can change during a match, but its identity, players,
-- commitments remain immutable across verified routing changes. Receipts are append-only.
GRANT SELECT ON central_identity.ranked_seasons, central_identity.ranked_hosts,
  central_identity.ranked_matches, central_identity.ranked_tickets TO central_runtime;
GRANT SELECT, INSERT (match_id, host_id) ON central_identity.ranked_host_aliases TO central_runtime;
GRANT SELECT, INSERT (match_id, host_id, seat, epoch, origin, version, digest, allowed),
  UPDATE (epoch, origin, version, digest, allowed)
  ON central_identity.ranked_host_permissions TO central_runtime;
GRANT INSERT (host_id, ranked_epoch, capacity, seen_at, compatibility), UPDATE (ranked_epoch, capacity, seen_at, continuity_since, compatibility)
  ON central_identity.ranked_hosts TO central_runtime;
GRANT INSERT (season_id, format, account_a, account_b, commitment_a, commitment_b, host_id, ranked_epoch, origin),
  UPDATE (claimed_a, claimed_b, game_id, start_fingerprint, deck_a, deck_b, state, outcome, finished_at, lease_until, host_id, ranked_epoch, origin)
  ON central_identity.ranked_matches TO central_runtime;
GRANT INSERT (account_id, request_id, season_id, format, commitment, queue_elo, replicable), UPDATE (state, match_id)
  ON central_identity.ranked_tickets TO central_runtime;
GRANT SELECT, INSERT, UPDATE (elo, queue_elo, score, placement, allowance, games, wins, losses, draws, league, division, last_at)
  ON central_identity.ranked_ratings TO central_runtime;
GRANT SELECT, INSERT, UPDATE (elo, anchor) ON central_identity.ranked_deck_versions TO central_runtime;
GRANT SELECT, INSERT (match_id,standby_id,standby_epoch,compatibility),
  UPDATE (seed,generation,standby_id,standby_epoch,primary_until,standby_until,paused_until,phase,position,head_digest,state_digest,opening_chain,setup,initial_pools,initial_evidence,evidence,journal_bytes,retire_at)
  ON central_identity.ranked_replicas TO central_runtime;
GRANT SELECT, INSERT, DELETE, UPDATE (evidence) ON central_identity.ranked_commands TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.ranked_witnesses TO central_runtime;
GRANT SELECT, INSERT, DELETE, UPDATE (checkpoint) ON central_identity.ranked_clocks TO central_runtime;
GRANT SELECT, INSERT, UPDATE (elo, games, season_games, wins, losses, draws, league, division, last_at)
  ON central_identity.ranked_deck_ratings TO central_runtime;
GRANT SELECT, INSERT ON central_identity.ranked_receipts TO central_runtime;
GRANT SELECT, INSERT, UPDATE (published_on, finalized) ON central_identity.ranked_publications TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.ranked_champions TO central_runtime;
GRANT SELECT, INSERT, UPDATE (place) ON central_identity.ranked_badge_peaks TO central_runtime;

-- Only the operator stages a legacy source. The authenticated transfer advances or retires it.
GRANT SELECT, DELETE, UPDATE (next_page) ON central_identity.legacy_deck_transfers TO central_runtime;

-- Only the offline publisher provisions campaign data. Players can insert
-- their first solution/claim and present their selected deck slots through Central.
GRANT SELECT ON central_identity.playmat_framing,central_identity.advent_art,central_identity.advent_campaigns,central_identity.advent_doors TO central_runtime;
GRANT SELECT, INSERT (account_id,year,door) ON central_identity.advent_solutions TO central_runtime;
GRANT SELECT, INSERT (account_id,year,door,art_id) ON central_identity.advent_entitlements TO central_runtime;
GRANT SELECT ON central_identity.cosmetic_collection, central_identity.ranked_cosmetic_catalog,
  central_identity.ranked_cosmetic_unlocks TO central_runtime;
GRANT SELECT, INSERT(account_id,art_id,advent_id,league,division) ON central_identity.cosmetic_equip_rights TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.advent_ranked_looks TO central_runtime;
GRANT SELECT, DELETE, INSERT (proof,account_id,gateway_id,sid,game,seat,sleeve,playmat),
      UPDATE (confirmed,expires_at) ON central_identity.advent_table_looks TO central_runtime;
GRANT SELECT, DELETE, INSERT (account_id,gateway_id,game,seat,stream,sleeve,playmat)
  ON central_identity.advent_table_outfits TO central_runtime;
-- Public card catalog (operator-ingested, read-only at runtime).
GRANT SELECT ON central_identity.card_printings, central_identity.card_search_names TO central_runtime;
