GRANT USAGE ON SCHEMA central_identity TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
            GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions,
                central_identity.gateway_tickets, central_identity.private_objects TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.migration_archives TO central_runtime;
            GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
            GRANT SELECT, INSERT, UPDATE (enabled) ON central_identity.gateway_approvals TO central_runtime;
            GRANT SELECT, INSERT, UPDATE (revoked_at, last_used_at) ON central_identity.gateway_tokens TO central_runtime;
            GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins, central_identity.gateway_leases TO central_runtime;
            GRANT UPDATE (expires_at) ON central_identity.gateway_leases TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
                central_identity.terms_acceptances TO central_runtime;
            GRANT SELECT, UPDATE (uses_left) ON central_identity.beta_keys TO central_runtime;
            GRANT SELECT, INSERT, DELETE ON central_identity.account_roles TO central_runtime;
            GRANT SELECT, INSERT ON central_identity.audit_events TO central_runtime;
            GRANT USAGE ON SCHEMA central_legacy TO central_legacy;
            GRANT SELECT, DELETE ON central_legacy.accounts TO central_legacy;
            GRANT UPDATE (failures, locked_until) ON central_legacy.accounts TO central_legacy;
            GRANT SELECT ON central_legacy.settings TO central_legacy