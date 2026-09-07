use super::*;
use crate::db::test_support as fixture;

fn item(db: &mut Connection, stock: i64) -> Product {
    create_product(
        db,
        CreateProductInput {
            request_id: Uuid::new_v4().to_string(),
            name: "Chargeur".into(),
            price: 2_000,
            initial_stock: stock,
        },
    )
    .unwrap()
}
fn sale_input(db: &Connection, product: &Product, quantity: i64) -> CreateSaleInput {
    CreateSaleInput {
        request_id: Uuid::new_v4().to_string(),
        lines: vec![SaleLineInput {
            product_id: product.id.clone(),
            quantity,
            unit_price: 2_000,
        }],
        account_id: fixture::id(db, "Espèces"),
        occurred_at: "2026-09-07".into(),
        note: None,
    }
}
fn receipt_input(db: &Connection, product: &Product, quantity: i64) -> ReceiveStockInput {
    ReceiveStockInput {
        request_id: Uuid::new_v4().to_string(),
        product_id: product.id.clone(),
        quantity,
        amount: 6_000,
        account_id: fixture::id(db, "Espèces"),
        occurred_at: "2026-09-07".into(),
        note: None,
    }
}
fn counts(db: &Connection) -> Vec<i64> {
    [
        "products",
        "product_operations",
        "product_operation_lines",
        "stock_movements",
        "journal_entries",
        "audit_events",
        "stock_requests",
    ]
    .iter()
    .map(|table| {
        db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    })
    .collect()
}
fn cancel(db: &mut Connection, op: &ProductOperation) -> AppResult<ProductOperation> {
    cancel_operation(
        db,
        CancelProductOperationInput {
            request_id: Uuid::new_v4().to_string(),
            operation_id: op.id.clone(),
            reason: "Erreur de saisie".into(),
        },
    )
}

#[test]
fn sales_receipts_inventory_and_reports_count_money_once() {
    let mut db = fixture::multi_database();
    let p = item(&mut db, 10);
    let start = db::get_dashboard(&db).unwrap().expected_capital;
    let input = sale_input(&db, &p, 2);
    create_sale(&mut db, input).unwrap();
    assert_eq!(product(&db, &p.id, true).unwrap().stock, 8);
    assert_eq!(
        db::get_dashboard(&db).unwrap().expected_capital,
        start + 4_000
    );
    let input = receipt_input(&db, &p, 5);
    receive_stock(&mut db, input).unwrap();
    assert_eq!(product(&db, &p.id, true).unwrap().stock, 13);
    assert_eq!(
        db::get_dashboard(&db).unwrap().expected_capital,
        start - 2_000
    );
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
        (4_000, -6_000)
    );
    assert_eq!(report.journal.len(), 2);
    assert!(report.journal.iter().all(|j| j.product_operation.is_some()));
    let mut balances = fixture::balances(&db);
    balances
        .iter_mut()
        .find(|b| b.account_id == fixture::id(&db, "Espèces"))
        .unwrap()
        .amount -= 2_000;
    let closed = fixture::close(&mut db, balances);
    assert_eq!(closed.variance, 0);
    assert_eq!(
        db::get_dashboard(&db).unwrap().expected_capital,
        start - 2_000
    );
    assert_eq!(
        db::get_dashboard(&db).unwrap().journal_net_since_inventory,
        0
    );
}

#[test]
fn baskets_custom_prices_accounts_and_immutable_snapshots() {
    let mut db = fixture::multi_database();
    let first = item(&mut db, 10);
    let second = item(&mut db, 10);
    for account in ["Espèces", "Orange 2", "Wave 2", "Djamo 1"] {
        let mut input = sale_input(&db, &first, 1);
        input.account_id = fixture::id(&db, account);
        input.lines[0].unit_price = 1_750;
        input.lines.push(SaleLineInput {
            product_id: second.id.clone(),
            quantity: 2,
            unit_price: 500,
        });
        let op = create_sale(&mut db, input).unwrap();
        assert_eq!(op.amount, 2_750);
        assert_eq!(op.account_snapshot.name, account);
    }
    update_product(
        &mut db,
        UpdateProductInput {
            request_id: Uuid::new_v4().to_string(),
            product_id: first.id.clone(),
            name: "Nouveau nom".into(),
            price: 4_000,
        },
    )
    .unwrap();
    let ops = operations(&db).unwrap();
    assert!(ops
        .iter()
        .all(|o| o.lines[0].product_name == "Chargeur" && o.lines[0].unit_price == Some(1_750)));
    assert_eq!(product(&db, &first.id, true).unwrap().stock, 6);
    assert_eq!(product(&db, &second.id, true).unwrap().stock, 2);
}

#[test]
fn retries_are_idempotent_and_reusing_a_key_with_other_data_is_rejected() {
    let mut db = fixture::multi_database();
    let create = CreateProductInput {
        request_id: Uuid::new_v4().to_string(),
        name: "Câble".into(),
        price: 500,
        initial_stock: 10,
    };
    let p = create_product(&mut db, create.clone()).unwrap();
    assert_eq!(create_product(&mut db, create).unwrap().id, p.id);
    let mut input = sale_input(&db, &p, 2);
    let op = create_sale(&mut db, input.clone()).unwrap();
    let saved = counts(&db);
    assert_eq!(create_sale(&mut db, input.clone()).unwrap().id, op.id);
    assert_eq!(saved, counts(&db));
    input.lines[0].quantity = 3;
    assert!(create_sale(&mut db, input).is_err());
    assert_eq!(saved, counts(&db));
    let receive = receipt_input(&db, &p, 1);
    receive_stock(&mut db, receive.clone()).unwrap();
    let saved = counts(&db);
    receive_stock(&mut db, receive).unwrap();
    assert_eq!(saved, counts(&db));
}

#[test]
fn invalid_and_partial_operations_leave_no_writes() {
    let mut db = fixture::multi_database();
    let p = item(&mut db, 10);
    let empty = item(&mut db, 0);
    let initial = counts(&db);
    let valid = sale_input(&db, &p, 2);
    let mut cases = vec![];
    let mut input = valid.clone();
    input.lines.clear();
    cases.push(input);
    let mut input = valid.clone();
    input.lines[0].quantity = 0;
    cases.push(input);
    let mut input = valid.clone();
    input.lines[0].quantity = -1;
    cases.push(input);
    let mut input = valid.clone();
    input.lines[0].unit_price = 0;
    cases.push(input);
    let mut input = valid.clone();
    input.lines[0].unit_price = MAX_INTEGER;
    cases.push(input);
    let mut input = valid.clone();
    input.lines[0].quantity = 11;
    cases.push(input);
    let mut input = valid.clone();
    input.lines.push(input.lines[0].clone());
    cases.push(input);
    let mut input = valid.clone();
    input.account_id = "cash".into();
    cases.push(input);
    let mut input = valid.clone();
    input.occurred_at = "pas une date".into();
    cases.push(input);
    // The second line fails after the first line and the journal were written.
    let mut input = valid.clone();
    input.lines.push(SaleLineInput {
        product_id: empty.id,
        quantity: 1,
        unit_price: 500,
    });
    cases.push(input);
    for input in cases {
        assert!(create_sale(&mut db, input).is_err());
        assert_eq!(counts(&db), initial);
        assert_eq!(product(&db, &p.id, true).unwrap().stock, 10);
    }
    db.execute_batch("CREATE TRIGGER fail_stock_audit BEFORE INSERT ON audit_events WHEN NEW.action='product_sale_created' BEGIN SELECT RAISE(ABORT,'Simulated failure'); END;").unwrap();
    assert!(create_sale(&mut db, valid).is_err());
    assert_eq!(counts(&db), initial);
    assert_eq!(product(&db, &p.id, true).unwrap().stock, 10);
    let fractional = json!({"requestId":Uuid::new_v4().to_string(),"lines":[{"productId":p.id,"quantity":0.5,"unitPrice":100}],"accountId":"cash","occurredAt":"2026-09-07"});
    assert!(serde_json::from_value::<CreateSaleInput>(fractional).is_err());
}

#[test]
fn journal_reversal_restores_stock_even_for_archived_products() {
    let mut db = fixture::multi_database();
    let p = item(&mut db, 2);
    let input = sale_input(&db, &p, 2);
    let op = create_sale(&mut db, input).unwrap();
    archive_product(
        &mut db,
        ArchiveProductInput {
            request_id: Uuid::new_v4().to_string(),
            product_id: p.id.clone(),
        },
    )
    .unwrap();
    db::reverse_journal_entry(
        &mut db,
        ReverseEntryInput {
            entry_id: op.journal_entry_id.clone(),
            reason: "Vente annulée".into(),
        },
    )
    .unwrap();
    assert_eq!(product(&db, &p.id, true).unwrap().stock, 2);
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 5_000_000);
    assert!(operation(&db, &op.id).unwrap().cancelled_at.is_some());
    let saved = counts(&db);
    assert!(cancel(&mut db, &op).is_err());
    assert_eq!(counts(&db), saved);
}

#[test]
fn receipt_cancellation_checks_stock_and_cancellation_retries_are_safe() {
    let mut db = fixture::multi_database();
    let p = item(&mut db, 0);
    let input = receipt_input(&db, &p, 5);
    let receipt = receive_stock(&mut db, input).unwrap();
    let input = sale_input(&db, &p, 2);
    let sale = create_sale(&mut db, input).unwrap();
    let saved = counts(&db);
    assert!(cancel(&mut db, &receipt).is_err());
    assert_eq!(counts(&db), saved);
    cancel(&mut db, &sale).unwrap();
    let input = CancelProductOperationInput {
        request_id: Uuid::new_v4().to_string(),
        operation_id: receipt.id,
        reason: "Réception incorrecte".into(),
    };
    cancel_operation(&mut db, input.clone()).unwrap();
    let saved = counts(&db);
    cancel_operation(&mut db, input).unwrap();
    assert_eq!(counts(&db), saved);
    assert_eq!(product(&db, &p.id, true).unwrap().stock, 0);
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 5_000_000);
}

#[test]
fn physical_counts_initial_stock_and_archiving_have_no_money_effect() {
    let mut db = fixture::multi_database();
    let p = item(&mut db, 10);
    let input = ArchiveProductInput {
        request_id: Uuid::new_v4().to_string(),
        product_id: p.id.clone(),
    };
    assert!(archive_product(&mut db, input.clone()).is_err());
    let mut adjust = AdjustStockInput {
        request_id: Uuid::new_v4().to_string(),
        product_id: p.id.clone(),
        quantity: 0,
        reason: "".into(),
    };
    assert!(adjust_stock(&mut db, adjust.clone()).is_err());
    adjust.reason = "Articles abîmés".into();
    adjust_stock(&mut db, adjust.clone()).unwrap();
    adjust_stock(&mut db, adjust).unwrap();
    archive_product(&mut db, input).unwrap();
    assert!(db::list_journal_entries(&db, None).unwrap().is_empty());
    assert_eq!(db::get_dashboard(&db).unwrap().expected_capital, 5_000_000);
    assert_eq!(movements(&db, Some(&p.id)).unwrap().len(), 2);
    let input = sale_input(&db, &p, 1);
    assert!(create_sale(&mut db, input).is_err());
}

#[test]
fn migration_from_v2_preserves_history_and_runs_once() {
    let db = Connection::open_in_memory().unwrap();
    fixture::legacy_database(&db);
    db.pragma_update(None, "foreign_keys", false).unwrap();
    db.execute_batch(include_str!("migration_v2.sql")).unwrap();
    db.pragma_update(None, "foreign_keys", true).unwrap();
    let before = fixture::historical_data(&db);
    db::migrate(&db).unwrap();
    assert_eq!(db::schema_version(&db).unwrap(), 3);
    assert_eq!(before, fixture::historical_data(&db));
    assert!(products(&db).unwrap().is_empty());
    let saved = counts(&db);
    db::migrate(&db).unwrap();
    assert_eq!(counts(&db), saved);
    db::integrity_check(&db).unwrap();
}
