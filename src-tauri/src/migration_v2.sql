CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL CHECK(provider IN ('orange_money','wave','djamo','cash')),
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    name_key TEXT NOT NULL,
    identifier TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    created_at TEXT NOT NULL,
    UNIQUE(provider, name_key),
    CHECK(provider != 'cash' OR active = 1)
);
CREATE UNIQUE INDEX one_cash_account ON accounts(provider) WHERE provider = 'cash';
INSERT INTO accounts(id, provider, name, name_key, created_at)
SELECT 'legacy-' || p.provider, p.provider, p.name, lower(p.name), b.created_at
FROM business_settings b CROSS JOIN (
    SELECT 'orange_money' AS provider, 'Orange Money — Principal' AS name
    UNION ALL SELECT 'wave', 'Wave — Principal'
    UNION ALL SELECT 'djamo', 'Djamo — Principal'
    UNION ALL SELECT 'cash', 'Espèces'
) p;
CREATE TABLE inventory_account_balances (
    inventory_id TEXT NOT NULL REFERENCES inventories(id),
    account_id TEXT NOT NULL REFERENCES accounts(id),
    provider TEXT NOT NULL,
    account_name TEXT NOT NULL,
    account_identifier TEXT,
    amount INTEGER NOT NULL CHECK(amount >= 0),
    legacy INTEGER NOT NULL DEFAULT 0 CHECK(legacy IN (0,1)),
    PRIMARY KEY(inventory_id, account_id)
);
INSERT INTO inventory_account_balances
SELECT i.id, a.id, a.provider, a.name, a.identifier,
       CASE a.provider WHEN 'orange_money' THEN i.orange_money WHEN 'wave' THEN i.wave
       WHEN 'djamo' THEN i.djamo ELSE i.cash END, 1
FROM inventories i CROSS JOIN accounts a;
ALTER TABLE journal_entries ADD COLUMN account_id TEXT REFERENCES accounts(id);
ALTER TABLE journal_entries ADD COLUMN account_name TEXT;
ALTER TABLE journal_entries ADD COLUMN account_identifier TEXT;
UPDATE journal_entries SET account_id = 'legacy-' || payment_account,
    account_name = (SELECT name FROM accounts WHERE id = 'legacy-' || payment_account);
ALTER TABLE debt_payments ADD COLUMN account_id TEXT REFERENCES accounts(id);
ALTER TABLE debt_payments ADD COLUMN account_name TEXT;
ALTER TABLE debt_payments ADD COLUMN account_identifier TEXT;
UPDATE debt_payments SET account_id = 'legacy-' || account,
    account_name = (SELECT name FROM accounts WHERE id = 'legacy-' || account);
CREATE TABLE debts_v2 (
    id TEXT PRIMARY KEY,
    customer_name TEXT NOT NULL,
    phone TEXT NOT NULL,
    provider TEXT NOT NULL CHECK(provider IN ('orange_money','wave','djamo')),
    principal INTEGER NOT NULL CHECK(principal > 0),
    remaining INTEGER NOT NULL CHECK(remaining >= 0),
    issued_at TEXT NOT NULL,
    due_date TEXT,
    note TEXT,
    status TEXT NOT NULL CHECK(status IN ('open','partial','paid','cancelled')),
    cancellation_reason TEXT,
    created_at TEXT NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    account_name TEXT NOT NULL,
    account_identifier TEXT
);
INSERT INTO debts_v2
SELECT d.*, 'legacy-' || d.provider, a.name, a.identifier
FROM debts d JOIN accounts a ON a.id = 'legacy-' || d.provider;
DROP TABLE debts;
ALTER TABLE debts_v2 RENAME TO debts;
CREATE INDEX idx_debts_phone ON debts(phone);
CREATE INDEX idx_debts_status ON debts(status);
CREATE INDEX idx_journal_account ON journal_entries(account_id);
CREATE INDEX idx_debts_account ON debts(account_id);
CREATE INDEX idx_payments_account ON debt_payments(account_id);
CREATE INDEX idx_inventory_account ON inventory_account_balances(account_id);
CREATE TRIGGER journal_account_required BEFORE INSERT ON journal_entries
WHEN NEW.account_id IS NULL OR NEW.account_name IS NULL
BEGIN SELECT RAISE(ABORT, 'Compte obligatoire'); END;
CREATE TRIGGER payment_account_required BEFORE INSERT ON debt_payments
WHEN NEW.account_id IS NULL OR NEW.account_name IS NULL
BEGIN SELECT RAISE(ABORT, 'Compte obligatoire'); END;
CREATE TRIGGER journal_account_valid BEFORE INSERT ON journal_entries
WHEN NOT EXISTS(SELECT 1 FROM accounts WHERE id=NEW.account_id AND provider=NEW.payment_account
    AND (active=1 OR NEW.reverses_id IS NOT NULL))
BEGIN SELECT RAISE(ABORT, 'Compte invalide ou archivé'); END;
CREATE TRIGGER debt_account_valid BEFORE INSERT ON debts
WHEN NOT EXISTS(SELECT 1 FROM accounts WHERE id=NEW.account_id AND provider=NEW.provider AND active=1)
BEGIN SELECT RAISE(ABORT, 'Compte invalide ou archivé'); END;
CREATE TRIGGER payment_account_valid BEFORE INSERT ON debt_payments
WHEN NOT EXISTS(SELECT 1 FROM accounts WHERE id=NEW.account_id AND provider=NEW.account AND active=1)
BEGIN SELECT RAISE(ABORT, 'Compte invalide ou archivé'); END;
CREATE TRIGGER inventory_balance_provider BEFORE INSERT ON inventory_account_balances
WHEN NOT EXISTS(SELECT 1 FROM accounts WHERE id=NEW.account_id AND provider=NEW.provider AND active=1)
BEGIN SELECT RAISE(ABORT, 'Compte invalide ou archivé'); END;
CREATE TRIGGER inventories_no_update BEFORE UPDATE ON inventories
BEGIN SELECT RAISE(ABORT, 'Inventaire clôturé immuable'); END;
CREATE TRIGGER inventories_no_delete BEFORE DELETE ON inventories
BEGIN SELECT RAISE(ABORT, 'Inventaire clôturé immuable'); END;
CREATE TRIGGER balances_no_update BEFORE UPDATE ON inventory_account_balances
BEGIN SELECT RAISE(ABORT, 'Solde clôturé immuable'); END;
CREATE TRIGGER balances_no_delete BEFORE DELETE ON inventory_account_balances
BEGIN SELECT RAISE(ABORT, 'Solde clôturé immuable'); END;
CREATE TRIGGER journal_no_update BEFORE UPDATE ON journal_entries
BEGIN SELECT RAISE(ABORT, 'Utilisez une contre-écriture'); END;
CREATE TRIGGER journal_no_delete BEFORE DELETE ON journal_entries
BEGIN SELECT RAISE(ABORT, 'Utilisez une contre-écriture'); END;
CREATE TRIGGER payments_no_update BEFORE UPDATE ON debt_payments
BEGIN SELECT RAISE(ABORT, 'Remboursement immuable'); END;
CREATE TRIGGER payments_no_delete BEFORE DELETE ON debt_payments
BEGIN SELECT RAISE(ABORT, 'Remboursement immuable'); END;
CREATE TRIGGER debt_origin_immutable BEFORE UPDATE OF id,provider,account_id,account_name,account_identifier,principal,issued_at,created_at ON debts
BEGIN SELECT RAISE(ABORT, 'Origine de la dette immuable'); END;
CREATE TRIGGER debts_no_delete BEFORE DELETE ON debts
BEGIN SELECT RAISE(ABORT, 'Annulez la dette par correction'); END;
CREATE TRIGGER audit_no_update BEFORE UPDATE ON audit_events
BEGIN SELECT RAISE(ABORT, 'Audit immuable'); END;
CREATE TRIGGER audit_no_delete BEFORE DELETE ON audit_events
BEGIN SELECT RAISE(ABORT, 'Audit immuable'); END;
CREATE TRIGGER account_identity_immutable BEFORE UPDATE OF id, provider ON accounts
BEGIN SELECT RAISE(ABORT, 'Identité du compte immuable'); END;
CREATE TRIGGER accounts_no_delete BEFORE DELETE ON accounts
BEGIN SELECT RAISE(ABORT, 'Archivez le compte au lieu de le supprimer'); END;
PRAGMA user_version = 2;
