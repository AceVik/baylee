-- The runtime role's privileges, exactly those its startup check requires
-- (docs/operations.md §"Provisioning"). Idempotent: the deploy hook
-- (ops/deploy-hook) runs it after every migration. A migration that adds a
-- table adds its grant here.
GRANT USAGE ON SCHEMA central_identity TO central_runtime;
GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions,
  central_identity.gateway_tickets, central_identity.private_objects TO central_runtime;
GRANT SELECT, INSERT ON central_identity.migration_archives TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
-- Administrators approve, enable and disable gateways on the website
-- (docs/administration.md §"Gateways"); an approval is never deleted.
GRANT SELECT, INSERT, UPDATE (enabled) ON central_identity.gateway_approvals TO central_runtime;
-- Gateway tokens (docs/gateway-registry.md §"Gateway tokens"): stored by
-- digest; afterwards only their revocation and last use change.
GRANT SELECT, INSERT, UPDATE (revoked_at, last_used_at) ON central_identity.gateway_tokens TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins, central_identity.gateway_leases TO central_runtime;
-- A renewed session moves its leases' expiry with it, and nothing else of them.
GRANT UPDATE (expires_at) ON central_identity.gateway_leases TO central_runtime;
GRANT SELECT, INSERT ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
  central_identity.terms_acceptances TO central_runtime;
-- A key's uses are spent by enrollment; issuing and revoking keys stay with the operator.
GRANT SELECT, UPDATE (uses_left) ON central_identity.beta_keys TO central_runtime;
-- Administrators' decisions are the runtime's (docs/administration.md): it
-- grants and removes roles, and writes the audit, which it can neither rewrite
-- nor delete; the operator's sweep deletes events after three years.
GRANT SELECT, INSERT, DELETE ON central_identity.account_roles TO central_runtime;
GRANT SELECT, INSERT ON central_identity.audit_events TO central_runtime;
-- Account-owned contacts: immutable blocks and bounded per-account budgets.
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.social_contacts TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.social_blocks TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.social_budgets TO central_runtime;
-- Encrypted chat: immutable public keys, signed ciphertext and delivery wraps.
GRANT SELECT, INSERT, DELETE ON central_identity.chat_devices, central_identity.chat_messages,
  central_identity.chat_deliveries TO central_runtime;
GRANT USAGE ON SEQUENCE central_identity.chat_deliveries_seq_seq TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.chat_groups, central_identity.chat_members TO central_runtime;
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
