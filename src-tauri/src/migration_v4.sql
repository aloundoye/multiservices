CREATE TABLE custody_customers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    phone TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    balance INTEGER NOT NULL DEFAULT 0 CHECK(balance BETWEEN 0 AND 9007199254740991),
    created_at TEXT NOT NULL
);
CREATE TABLE custody_movements (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    customer_id TEXT NOT NULL REFERENCES custody_customers(id),
    customer_name TEXT NOT NULL,
    customer_phone TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('deposit','withdrawal','opening','reversal')),
    delta INTEGER NOT NULL CHECK(delta != 0 AND delta BETWEEN -9007199254740991 AND 9007199254740991),
    capital_adjustment INTEGER NOT NULL DEFAULT 0,
    balance_after INTEGER NOT NULL CHECK(balance_after BETWEEN 0 AND 9007199254740991),
    account_id TEXT REFERENCES accounts(id),
    account_json TEXT,
    occurred_at TEXT NOT NULL,
    posted_at TEXT NOT NULL,
    operator TEXT NOT NULL,
    note TEXT,
    reverses_id TEXT UNIQUE REFERENCES custody_movements(id)
);
CREATE INDEX idx_custody_customer ON custody_movements(customer_id, sequence);
CREATE INDEX idx_custody_account ON custody_movements(account_id, posted_at);
CREATE TABLE custody_requests (
    id TEXT PRIMARY KEY,
    action TEXT NOT NULL,
    input_json TEXT NOT NULL,
    result_json TEXT NOT NULL
);
ALTER TABLE inventories ADD COLUMN custody_total INTEGER CHECK(custody_total >= 0);
ALTER TABLE inventories ADD COLUMN custody_sequence INTEGER NOT NULL DEFAULT 0;
CREATE TABLE inventory_custody_balances (
    inventory_id TEXT NOT NULL REFERENCES inventories(id),
    customer_id TEXT NOT NULL REFERENCES custody_customers(id),
    customer_name TEXT NOT NULL,
    customer_phone TEXT,
    balance INTEGER NOT NULL CHECK(balance > 0),
    PRIMARY KEY(inventory_id, customer_id)
);
PRAGMA user_version = 4;
CREATE TRIGGER custody_movements_no_update BEFORE UPDATE ON custody_movements
BEGIN SELECT RAISE(ABORT, 'Utilisez un contre-mouvement'); END;
CREATE TRIGGER custody_movements_no_delete BEFORE DELETE ON custody_movements
BEGIN SELECT RAISE(ABORT, 'Utilisez un contre-mouvement'); END;
CREATE TRIGGER custody_snapshots_no_update BEFORE UPDATE ON inventory_custody_balances
BEGIN SELECT RAISE(ABORT, 'Dépôt clôturé immuable'); END;
CREATE TRIGGER custody_snapshots_no_delete BEFORE DELETE ON inventory_custody_balances
BEGIN SELECT RAISE(ABORT, 'Dépôt clôturé immuable'); END;
CREATE TRIGGER custody_identity_immutable BEFORE UPDATE OF id,created_at ON custody_customers
BEGIN SELECT RAISE(ABORT, 'Identité du client immuable'); END;
CREATE TRIGGER custody_customers_no_delete BEFORE DELETE ON custody_customers
BEGIN SELECT RAISE(ABORT, 'Archivez le client'); END;
