//! Isolated fixtures: never access the real application data or OS keyring.
use super::*;

pub fn legacy_database(connection: &Connection) {
    migrate_v1(connection).unwrap();
    connection.execute_batch(
        r#"INSERT INTO business_settings VALUES(1,'Boutique historique','XOF','Africa/Dakar',240,15,'2026-01-01T08:00:00Z');
        INSERT INTO inventories VALUES('opening','opening','2026-01-01T08:00:00Z',1500000,1200000,800000,1500000,0,5000000,5000000,5000000,0,NULL,NULL);
        INSERT INTO inventories VALUES('regular','regular','2026-01-01T12:00:00Z',1520000,1150000,800000,1520000,30000,4990000,5000000,5020000,20000,'autre','Ancien écart');
        INSERT INTO journal_entries VALUES('sale','sale',100000,100000,'cash','2026-01-01','2026-01-01T09:00:00Z','Ticket 45','Vente',NULL);
        INSERT INTO journal_entries VALUES('reverse','reversal',100000,-100000,'cash','2026-01-01','2026-01-01T09:30:00Z','Correction sale','Erreur','sale');
        INSERT INTO journal_entries VALUES('correction','inventory_correction',25000,-25000,'orange_money','2026-01-01','2026-01-01T10:00:00Z','inventory:opening','Comptage',NULL);
        INSERT INTO debts VALUES('debt','Awa Ndiaye','771234567','wave',50000,30000,'2026-01-01',NULL,'Transfert','partial',NULL,'2026-01-01T10:30:00Z');
        INSERT INTO debt_payments VALUES('payment','debt',20000,'cash','2026-01-01','Acompte','2026-01-01T11:00:00Z');
        INSERT INTO audit_events VALUES('audit-legacy','inventory_closed','inventory','regular','{"variance":20000}','2026-01-01T12:00:00Z');"#
    ).unwrap();
}

pub fn historical_data(connection: &Connection) -> Vec<Vec<rusqlite::types::Value>> {
    let queries = [
        "SELECT * FROM business_settings",
        "SELECT * FROM inventories ORDER BY id",
        "SELECT id,entry_type,amount,signed_amount,payment_account,occurred_at,posted_at,reference,note,reverses_id FROM journal_entries ORDER BY id",
        "SELECT id,customer_name,phone,provider,principal,remaining,issued_at,due_date,note,status,cancellation_reason,created_at FROM debts ORDER BY id",
        "SELECT id,debt_id,amount,account,paid_at,note,created_at FROM debt_payments ORDER BY id",
        "SELECT * FROM audit_events WHERE id='audit-legacy'",
    ];
    queries
        .into_iter()
        .flat_map(|sql| {
            let mut statement = connection.prepare(sql).unwrap();
            let columns = statement.column_count();
            statement
                .query_map([], |r| (0..columns).map(|i| r.get(i)).collect())
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        })
        .collect()
}

pub fn multi_setup() -> SetupInput {
    SetupInput {
        business_name: "Boutique SIM".into(),
        pin: "123456".into(),
        recovery_password: "une phrase de récupération solide".into(),
        initial_capital: 5_000_000,
        accounts: [
            ("orange_money", "Orange 1", 1_000_000),
            ("orange_money", "Orange 2", 500_000),
            ("wave", "Wave 1", 600_000),
            ("wave", "Wave 2", 600_000),
            ("djamo", "Djamo 1", 800_000),
            ("cash", "Espèces", 1_500_000),
        ]
        .into_iter()
        .map(|(provider, name, amount)| OpeningAccount {
            provider: provider.into(),
            name: name.into(),
            amount,
            identifier: Some(format!("SIM-{name}")),
        })
        .collect(),
    }
}

pub fn multi_database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    migrate(&db).unwrap();
    initialize_business(&mut db, &multi_setup()).unwrap();
    db
}

pub fn id(db: &Connection, name: &str) -> String {
    accounts::list(db)
        .unwrap()
        .into_iter()
        .find(|a| a.snapshot.name == name)
        .unwrap()
        .snapshot
        .account_id
}

pub fn balances(db: &Connection) -> Vec<AccountBalanceInput> {
    accounts::list(db)
        .unwrap()
        .into_iter()
        .filter(|a| a.active)
        .map(|a| AccountBalanceInput {
            account_id: a.snapshot.account_id,
            amount: a.last_balance.unwrap_or(0),
        })
        .collect()
}

pub fn journal(db: &mut Connection, id: &str) -> JournalEntry {
    create_journal_entry(
        db,
        CreateJournalEntryInput {
            entry_type: "sale".into(),
            amount: 10_000,
            account_id: id.into(),
            occurred_at: "2026-09-01".into(),
            reference: Some("Reçu test".into()),
            note: None,
        },
    )
    .unwrap()
}

pub fn debt(db: &mut Connection, id: &str) -> Debt {
    create_debt(
        db,
        CreateDebtInput {
            customer_name: "Awa Ndiaye".into(),
            phone: "771234567".into(),
            account_id: id.into(),
            amount: 50_000,
            issued_at: "2026-09-01".into(),
            due_date: None,
            note: None,
        },
    )
    .unwrap()
}

pub fn payment(db: &mut Connection, debt_id: &str, id: &str, amount: i64) -> Debt {
    record_debt_payment(
        db,
        RecordPaymentInput {
            debt_id: debt_id.into(),
            amount,
            account_id: id.into(),
            paid_at: "2026-09-01".into(),
            note: None,
        },
    )
    .unwrap()
}

pub fn close(db: &mut Connection, balances: Vec<AccountBalanceInput>) -> Inventory {
    close_inventory(
        db,
        CloseInventoryInput {
            balances,
            variance_category: Some("autre".into()),
            variance_note: Some("Rapprochement de test".into()),
        },
    )
    .unwrap()
}
