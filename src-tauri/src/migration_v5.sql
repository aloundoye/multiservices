CREATE TABLE debt_customers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    phone TEXT NOT NULL,
    active INTEGER NOT NULL CHECK(active IN (0,1)),
    created_at TEXT NOT NULL
);
ALTER TABLE debts ADD COLUMN customer_id TEXT REFERENCES debt_customers(id);
CREATE INDEX idx_debts_customer ON debts(customer_id, issued_at, created_at, id);
CREATE TABLE customer_repayments (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL REFERENCES debt_customers(id),
    customer_name TEXT NOT NULL,
    customer_phone TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK(amount BETWEEN 1 AND 9007199254740991),
    account_id TEXT NOT NULL REFERENCES accounts(id),
    account_json TEXT NOT NULL,
    paid_at TEXT NOT NULL,
    note TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE repayment_allocations (
    repayment_id TEXT NOT NULL REFERENCES customer_repayments(id),
    payment_id TEXT NOT NULL UNIQUE REFERENCES debt_payments(id),
    debt_id TEXT NOT NULL REFERENCES debts(id),
    amount INTEGER NOT NULL CHECK(amount BETWEEN 1 AND 9007199254740991),
    remaining_after INTEGER NOT NULL CHECK(remaining_after BETWEEN 0 AND 9007199254740991),
    position INTEGER NOT NULL,
    PRIMARY KEY(repayment_id, debt_id),
    UNIQUE(repayment_id, position)
);
CREATE INDEX idx_repayments_date ON customer_repayments(paid_at);
CREATE TABLE debt_requests (
    id TEXT PRIMARY KEY,
    action TEXT NOT NULL,
    input_json TEXT NOT NULL,
    result_json TEXT NOT NULL
);
CREATE TRIGGER debt_customers_identity BEFORE UPDATE OF id,created_at ON debt_customers
BEGIN SELECT RAISE(ABORT, 'Identité du client immuable'); END;
CREATE TRIGGER debt_customers_no_delete BEFORE DELETE ON debt_customers
BEGIN SELECT RAISE(ABORT, 'Archivez la fiche client'); END;
CREATE TRIGGER repayments_no_update BEFORE UPDATE ON customer_repayments
BEGIN SELECT RAISE(ABORT, 'Remboursement immuable'); END;
CREATE TRIGGER repayments_no_delete BEFORE DELETE ON customer_repayments
BEGIN SELECT RAISE(ABORT, 'Remboursement immuable'); END;
CREATE TRIGGER allocations_no_update BEFORE UPDATE ON repayment_allocations
BEGIN SELECT RAISE(ABORT, 'Répartition immuable'); END;
CREATE TRIGGER allocations_no_delete BEFORE DELETE ON repayment_allocations
BEGIN SELECT RAISE(ABORT, 'Répartition immuable'); END;
CREATE TRIGGER debt_requests_no_update BEFORE UPDATE ON debt_requests
BEGIN SELECT RAISE(ABORT, 'Requête immuable'); END;
CREATE TRIGGER debt_requests_no_delete BEFORE DELETE ON debt_requests
BEGIN SELECT RAISE(ABORT, 'Requête immuable'); END;
PRAGMA user_version = 5;
CREATE TRIGGER debt_customer_snapshot_immutable BEFORE UPDATE OF customer_id,customer_name,phone ON debts
WHEN OLD.customer_id IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'Client historique immuable'); END;
