CREATE TABLE products (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    price INTEGER NOT NULL CHECK(price > 0 AND price <= 9007199254740991),
    stock INTEGER NOT NULL CHECK(stock >= 0 AND stock <= 9007199254740991),
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    created_at TEXT NOT NULL,
    CHECK(active = 1 OR stock = 0)
);
CREATE TABLE product_operations (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('sale','receipt')),
    journal_entry_id TEXT NOT NULL UNIQUE REFERENCES journal_entries(id),
    cancelled_at TEXT,
    cancellation_reason TEXT
);
CREATE TABLE product_operation_lines (
    operation_id TEXT NOT NULL REFERENCES product_operations(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    product_name TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK(quantity > 0),
    unit_price INTEGER CHECK(unit_price > 0),
    total INTEGER NOT NULL CHECK(total > 0),
    PRIMARY KEY(operation_id, product_id)
);
CREATE TABLE stock_movements (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id),
    product_name TEXT NOT NULL,
    operation_id TEXT REFERENCES product_operations(id),
    kind TEXT NOT NULL CHECK(kind IN ('initial','sale','receipt','adjustment','cancellation')),
    quantity INTEGER NOT NULL,
    balance_after INTEGER NOT NULL CHECK(balance_after >= 0),
    reason TEXT,
    occurred_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_stock_product ON stock_movements(product_id, created_at);
CREATE TABLE stock_requests (
    id TEXT PRIMARY KEY,
    action TEXT NOT NULL,
    input_json TEXT NOT NULL,
    result_json TEXT NOT NULL
);
PRAGMA user_version = 3;
