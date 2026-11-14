ALTER TABLE @tower.verification_jobs ADD COLUMN failure text
    CHECK (failure IN ('invalid_record', 'budget', 'worker', 'protocol'));
