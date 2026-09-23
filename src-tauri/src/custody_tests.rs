use super::*;
use crate::db::test_support as fixture;

fn request() -> String {
    Uuid::new_v4().to_string()
}
fn client(db: &mut Connection, name: &str) -> CustodyCustomer {
    save_customer(
        db,
        SaveCustodyCustomerInput {
            request_id: request(),
            customer_id: None,
            name: name.into(),
            phone: Some("771234567".into()),
            active: true,
        },
    )
    .unwrap()
}
fn movement(
    client: &CustodyCustomer,
    account: &str,
    kind: &str,
    amount: i64,
) -> CreateCustodyMovementInput {
    CreateCustodyMovementInput {
        request_id: request(),
        customer_id: client.id.clone(),
        kind: kind.into(),
        amount,
        account_id: account.into(),
        occurred_at: "2026-09-21".into(),
        note: None,
    }
}
fn cancel(db: &mut Connection, m: &CustodyMovement) -> AppResult<CustodyMovement> {
    reverse(
        db,
        ReverseCustodyMovementInput {
            request_id: request(),
            movement_id: m.id.clone(),
            reason: "Erreur de saisie".into(),
        },
    )
}
fn reprise(c: &CustodyCustomer, amount: i64) -> CustodyOpeningInput {
    CustodyOpeningInput {
        request_id: request(),
        lines: vec![CustodyOpeningLine {
            customer_id: c.id.clone(),
            amount,
        }],
    }
}
fn setup_million() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    db::migrate(&db).unwrap();
    let mut setup = fixture::multi_setup();
    setup.initial_capital = 1_000_000;
    for a in &mut setup.accounts {
        a.amount = if a.provider == "cash" { 1_000_000 } else { 0 };
    }
    db::initialize_business(&mut db, &setup).unwrap();
    db
}

#[test]
fn opening_deposit_withdrawal_and_successive_inventories_preserve_own_capital() {
    let mut db = setup_million();
    let c = client(&mut db, "Awa");
    let opening_input = reprise(&c, 200_000);
    let preview = preview_opening(&db, &opening_input).unwrap();
    assert_eq!(
        (
            preview.total,
            preview.expected_capital,
            preview.corrected_capital
        ),
        (200_000, 1_000_000, 800_000)
    );
    let initial = opening(&mut db, opening_input.clone()).unwrap();
    assert_eq!(
        opening(&mut db, opening_input).unwrap()[0].id,
        initial[0].id
    );
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 800_000);
    assert_eq!(db::last_inventory(&db).unwrap().liquidity, 1_000_000);
    let wave = fixture::id(&db, "Wave 2");
    let cash = fixture::id(&db, "Espèces");
    record(&mut db, movement(&c, &wave, "deposit", 50_000)).unwrap();
    record(&mut db, movement(&c, &cash, "withdrawal", 80_000)).unwrap();
    let dashboard = db::get_dashboard(&db).unwrap();
    assert_eq!(
        (
            dashboard.custody_total,
            dashboard.expected_capital,
            dashboard.custody_customers_count
        ),
        (170_000, 800_000, 1)
    );
    assert!(db::list_journal_entries(&db, None).unwrap().is_empty());
    let mut balances = fixture::balances(&db);
    for b in &mut balances {
        if b.account_id == wave {
            b.amount = 50_000;
        }
        if b.account_id == cash {
            b.amount = 920_000;
        }
    }
    let closed = fixture::close(&mut db, balances.clone());
    assert_eq!(
        (closed.liquidity, closed.actual_total, closed.variance),
        (970_000, 800_000, 0)
    );
    assert_eq!(closed.custody_total, Some(170_000));
    assert_eq!(closed.custody_balances[0].balance, 170_000);
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 800_000);
    assert_eq!(fixture::close(&mut db, balances).variance, 0);
    // A reclassification reversal after its cutoff restores own capital exactly once.
    assert!(cancel(&mut db, &initial[0]).is_err()); // remaining < original opening
    record(&mut db, movement(&c, &cash, "deposit", 30_000)).unwrap();
    cancel(&mut db, &initial[0]).unwrap();
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 1_000_000);
    let historical = db::list_inventories(&db, None)
        .unwrap()
        .into_iter()
        .find(|i| i.id == closed.id)
        .unwrap();
    assert_eq!(historical.custody_total, Some(170_000));
    assert_eq!(historical.custody_balances[0].balance, 170_000);
}

#[test]
fn reversal_archive_rename_idempotency_and_accounts() {
    let mut db = fixture::multi_database();
    let c = client(&mut db, "Awa");
    let cash = fixture::id(&db, "Espèces");
    let mut input = movement(&c, &cash, "deposit", 1_000);
    let deposit = record(&mut db, input.clone()).unwrap();
    assert_eq!(record(&mut db, input.clone()).unwrap().id, deposit.id);
    input.amount = 2_000;
    assert!(record(&mut db, input).is_err());
    assert!(record(&mut db, movement(&c, &cash, "withdrawal", 1_001)).is_err());
    let mut edit = SaveCustodyCustomerInput {
        request_id: request(),
        customer_id: Some(c.id.clone()),
        name: "Awa Fall".into(),
        phone: None,
        active: false,
    };
    assert!(save_customer(&mut db, edit.clone()).is_err());
    edit.active = true;
    save_customer(&mut db, edit.clone()).unwrap();
    assert_eq!(movements(&db).unwrap()[0].customer_name, "Awa");
    let withdrawal = record(&mut db, movement(&c, &cash, "withdrawal", 1_000)).unwrap();
    assert_eq!(withdrawal.customer_name, "Awa Fall");
    edit.request_id = request();
    edit.active = false;
    save_customer(&mut db, edit).unwrap();
    assert!(cancel(&mut db, &deposit).is_err());
    cancel(&mut db, &withdrawal).unwrap();
    let current = customer(&db, &c.id, false).unwrap();
    assert!(current.active);
    assert_eq!(current.balance, 1_000);
    assert_eq!(current.total_withdrawn, 0);
    assert!(cancel(&mut db, &withdrawal).is_err());
    cancel(&mut db, &deposit).unwrap();
    assert_eq!(total(&db).unwrap(), 0);
    let account = accounts::create(
        &mut db,
        CreateAccountInput {
            provider: "wave".into(),
            name: "Compte vide".into(),
            identifier: None,
        },
    )
    .unwrap();
    record(
        &mut db,
        movement(&c, &account.snapshot.account_id, "deposit", 10),
    )
    .unwrap();
    assert!(accounts::set_active(&mut db, &account.snapshot.account_id, false).is_err());
}

#[test]
fn all_providers_multiple_clients_and_rollback() {
    let mut db = fixture::multi_database();
    let a = client(&mut db, "Awa");
    let b = client(&mut db, "Moussa");
    for account in accounts::list(&db).unwrap() {
        record(
            &mut db,
            movement(&a, &account.snapshot.account_id, "deposit", 100),
        )
        .unwrap();
    }
    assert_eq!(customer(&db, &a.id, true).unwrap().balance, 600);
    let cash = fixture::id(&db, "Espèces");
    assert!(record(&mut db, movement(&b, &cash, "withdrawal", 1)).is_err());
    let before = movements(&db).unwrap().len();
    db.execute_batch("CREATE TRIGGER fail_audit BEFORE INSERT ON audit_events WHEN NEW.action='custody_movement_recorded' BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let input = movement(&a, &cash, "deposit", 50);
    assert!(record(&mut db, input.clone()).is_err());
    assert_eq!(movements(&db).unwrap().len(), before);
    assert_eq!(total(&db).unwrap(), 600);
    db.execute_batch("DROP TRIGGER fail_audit;").unwrap();
    record(&mut db, input).unwrap();
    assert_eq!(total(&db).unwrap(), 650);
    let batch = CustodyOpeningInput {
        request_id: request(),
        lines: vec![
            CustodyOpeningLine {
                customer_id: a.id.clone(),
                amount: 200,
            },
            CustodyOpeningLine {
                customer_id: b.id.clone(),
                amount: 300,
            },
        ],
    };
    db.execute_batch("CREATE TRIGGER fail_second BEFORE INSERT ON custody_movements WHEN NEW.customer_name='Moussa' BEGIN SELECT RAISE(ABORT,'second line'); END;").unwrap();
    assert!(opening(&mut db, batch).is_err());
    assert_eq!(total(&db).unwrap(), 650);
}

#[test]
fn numeric_dates_and_archived_accounts_are_validated() {
    let mut db = fixture::multi_database();
    let c = client(&mut db, "Awa");
    let cash = fixture::id(&db, "Espèces");
    for amount in [0, -1, MAX + 1, i64::MAX] {
        assert!(record(&mut db, movement(&c, &cash, "deposit", amount)).is_err());
    }
    let mut input = movement(&c, &cash, "deposit", 10);
    input.occurred_at = "bad-date".into();
    assert!(record(&mut db, input).is_err());
    let account = accounts::create(
        &mut db,
        CreateAccountInput {
            provider: "wave".into(),
            name: "Archive".into(),
            identifier: None,
        },
    )
    .unwrap();
    accounts::set_active(&mut db, &account.snapshot.account_id, false).unwrap();
    assert!(record(
        &mut db,
        movement(&c, &account.snapshot.account_id, "deposit", 10)
    )
    .is_err());
    record(&mut db, movement(&c, &cash, "deposit", MAX)).unwrap();
    assert!(record(&mut db, movement(&c, &cash, "deposit", 1)).is_err());
    let c2 = client(&mut db, "Binta");
    assert!(record(&mut db, movement(&c2, &cash, "deposit", 1)).is_err());
    let serialized = serde_json::to_value(movement(&c, &cash, "deposit", 1)).unwrap();
    let mut fractional = serialized;
    fractional["amount"] = serde_json::json!(1.5);
    assert!(serde_json::from_value::<CreateCustodyMovementInput>(fractional).is_err());
}

#[test]
fn backdated_movements_reports_and_receivables_do_not_rewrite_inventory() {
    let mut db = fixture::multi_database();
    let c = client(&mut db, "Awa");
    let cash = fixture::id(&db, "Espèces");
    let first = opening(&mut db, reprise(&c, 200_000)).unwrap();
    let wave = fixture::id(&db, "Wave 1");
    let debt = fixture::debt(&mut db, &wave);
    let mut balances = fixture::balances(&db);
    for b in &mut balances {
        if b.account_id == wave {
            b.amount -= 50_000;
        }
    }
    let closed = fixture::close(&mut db, balances);
    assert_eq!(closed.variance, 0);
    assert_eq!(closed.actual_total, 4_800_000);
    let mut input = movement(&c, &cash, "deposit", 50_000);
    input.occurred_at = "2026-08-01".into();
    record(&mut db, input).unwrap();
    let mut out = movement(&c, &cash, "withdrawal", 80_000);
    out.occurred_at = "2026-08-15".into();
    record(&mut db, out).unwrap();
    let report = report(
        &db,
        &ReportFilters {
            from: Some("2026-08-10".into()),
            to: Some("2026-08-31".into()),
        },
    )
    .unwrap();
    assert_eq!(
        (report.opening_balance, report.closing_balance),
        (50_000, -30_000)
    ); // declared dates, opening reclassification registered later
    assert_eq!(report.movements.len(), 1);
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 4_800_000);
    fixture::payment(&mut db, &debt.id, &cash, 20_000);
    fixture::journal(&mut db, &cash);
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 4_810_000);
    assert!(cancel(&mut db, &first[0]).is_err());
    let old = db::list_inventories(&db, None)
        .unwrap()
        .into_iter()
        .find(|i| i.id == closed.id)
        .unwrap();
    assert_eq!(old.custody_total, Some(200_000));
    assert_eq!(old.receivables, 50_000);
}

#[test]
fn schema_three_upgrade_is_conditional_and_preserves_history() {
    let db = Connection::open_in_memory().unwrap();
    fixture::legacy_database(&db);
    db.pragma_update(None, "foreign_keys", false).unwrap();
    db.execute_batch(include_str!("migration_v2.sql")).unwrap();
    db.execute_batch(include_str!("migration_v3.sql")).unwrap();
    let before = fixture::historical_data(&db);
    db::migrate(&db).unwrap();
    db::migrate(&db).unwrap();
    assert_eq!(db::schema_version(&db).unwrap(), db::SCHEMA_VERSION);
    assert_eq!(before, fixture::historical_data(&db));
    assert!(db::list_inventories(&db, None)
        .unwrap()
        .iter()
        .all(|i| i.custody_total.is_none() && i.custody_balances.is_empty()));
    db::integrity_check(&db).unwrap();
}

#[test]
fn products_reclassification_and_immutable_ledger_reconcile_together() {
    let mut db = setup_million();
    let c = client(&mut db, "Awa");
    let cash = fixture::id(&db, "Espèces");
    let original = opening(&mut db, reprise(&c, 200_000)).unwrap().remove(0);
    let product = crate::stock::create_product(
        &mut db,
        CreateProductInput {
            request_id: request(),
            name: "Câble".into(),
            price: 2000,
            initial_stock: 10,
        },
    )
    .unwrap();
    crate::stock::create_sale(
        &mut db,
        CreateSaleInput {
            request_id: request(),
            lines: vec![SaleLineInput {
                product_id: product.id.clone(),
                quantity: 2,
                unit_price: 2000,
            }],
            account_id: cash.clone(),
            occurred_at: "2026-09-01".into(),
            note: None,
        },
    )
    .unwrap();
    crate::stock::receive_stock(
        &mut db,
        ReceiveStockInput {
            request_id: request(),
            product_id: product.id,
            quantity: 5,
            amount: 6000,
            account_id: cash.clone(),
            occurred_at: "2026-09-01".into(),
            note: None,
        },
    )
    .unwrap();
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 798_000);
    let mut balances = fixture::balances(&db);
    for b in &mut balances {
        if b.account_id == cash {
            b.amount = 998_000;
        }
    }
    let closed = fixture::close(&mut db, balances.clone());
    assert_eq!(closed.variance, 0);
    assert!(db
        .execute(
            "UPDATE custody_movements SET delta=1 WHERE id=?1",
            [&original.id]
        )
        .is_err());
    assert!(db
        .execute("DELETE FROM custody_movements WHERE id=?1", [&original.id])
        .is_err());
    assert!(db
        .execute(
            "UPDATE inventory_custody_balances SET balance=1 WHERE inventory_id=?1",
            [&closed.id]
        )
        .is_err());
    assert!(db
        .execute(
            "DELETE FROM inventory_custody_balances WHERE inventory_id=?1",
            [&closed.id]
        )
        .is_err());
    let reverse_input = ReverseCustodyMovementInput {
        request_id: request(),
        movement_id: original.id,
        reason: "Reprise non nécessaire".into(),
    };
    let reversed = reverse(&mut db, reverse_input.clone()).unwrap();
    assert_eq!(reverse(&mut db, reverse_input).unwrap().id, reversed.id);
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 998_000);
    assert_eq!(fixture::close(&mut db, balances.clone()).variance, 0);
    assert_eq!(fixture::close(&mut db, balances).actual_total, 998_000);
    let report = db::get_report(
        &db,
        ReportFilters {
            from: None,
            to: None,
        },
    )
    .unwrap();
    assert_eq!(
        (report.total_positive, report.total_negative),
        (4000, -6000)
    );
}

#[test]
fn failed_schema_four_upgrade_rolls_back_without_touching_legacy_data() {
    let db = Connection::open_in_memory().unwrap();
    fixture::legacy_database(&db);
    db.pragma_update(None, "foreign_keys", false).unwrap();
    db.execute_batch(include_str!("migration_v2.sql")).unwrap();
    db.execute_batch(include_str!("migration_v3.sql")).unwrap();
    // An unexpected table forces an error after the first table of v4 is created.
    db.execute_batch("CREATE TABLE custody_movements(unexpected TEXT);")
        .unwrap();
    let before = fixture::historical_data(&db);
    assert!(db::migrate(&db).is_err());
    assert_eq!(db::schema_version(&db).unwrap(), 3);
    assert_eq!(fixture::historical_data(&db), before);
    assert!(!db
        .prepare("SELECT 1 FROM sqlite_master WHERE name='custody_customers'")
        .unwrap()
        .exists([])
        .unwrap());
}
