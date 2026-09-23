use super::*;
use crate::db::test_support as fixture;

fn request() -> String {
    Uuid::new_v4().to_string()
}
pub(crate) fn client(c: &mut Connection, name: &str) -> DebtCustomer {
    save_customer(
        c,
        SaveDebtCustomerInput {
            request_id: request(),
            customer_id: None,
            name: name.into(),
            phone: "771234567".into(),
            active: true,
        },
    )
    .unwrap()
}
fn loan(c: &mut Connection, client: &DebtCustomer, amount: Money, date: &str) -> Debt {
    let account = fixture::id(c, "Wave 1");
    create_debt(
        c,
        CreateClientDebtInput {
            request_id: request(),
            customer_id: client.id.clone(),
            account_id: account,
            amount,
            issued_at: date.into(),
            due_date: Some(date.into()),
            note: None,
        },
    )
    .unwrap()
}
fn payment(
    c: &Connection,
    client: &DebtCustomer,
    amount: Money,
    date: &str,
) -> RecordCustomerRepaymentInput {
    let preview = preview(
        c,
        &RepaymentPreviewInput {
            customer_id: client.id.clone(),
            amount,
            paid_at: date.into(),
        },
    )
    .unwrap();
    RecordCustomerRepaymentInput {
        request_id: request(),
        customer_id: client.id.clone(),
        amount,
        account_id: fixture::id(c, "Espèces"),
        paid_at: date.into(),
        note: Some("Règlement".into()),
        preview_token: preview.token,
    }
}
fn snapshot(c: &Connection) -> serde_json::Value {
    json!({"debts":db::list_debts(c,None).unwrap(),"repayments":repayments(c,None).unwrap(),"clients":customers(c).unwrap(),"audit":db::list_audit_events(c,1000).unwrap(),"requests":c.query_row("SELECT COUNT(*) FROM debt_requests",[],|r| r.get::<_,i64>(0)).unwrap()})
}

#[test]
fn global_fifo_repayment_is_atomic_idempotent_and_keeps_capital_stable() {
    let mut c = fixture::multi_database();
    let client = client(&mut c, "Awa Fall");
    let first = loan(&mut c, &client, 20_000, "2026-01-01");
    let second = loan(&mut c, &client, 30_000, "2026-01-02");
    let input = payment(&c, &client, 35_000, "2026-02-01");
    let receipt = record_repayment(&mut c, input.clone()).unwrap();
    assert_eq!(
        receipt
            .allocations
            .iter()
            .map(|a| (&a.debt_id, a.amount, a.remaining_after))
            .collect::<Vec<_>>(),
        vec![
            (&first.id, 20_000, Some(0)),
            (&second.id, 15_000, Some(15_000))
        ]
    );
    assert_eq!(customers(&c).unwrap()[0].remaining, 15_000);
    assert_eq!(customers(&c).unwrap()[0].total_repaid, 35_000);
    assert_eq!(db::get_dashboard(&c).unwrap().expected_capital, 5_000_000);
    assert!(db::list_journal_entries(&c, None).unwrap().is_empty());
    let after = snapshot(&c);
    assert_eq!(
        record_repayment(&mut c, input.clone()).unwrap().id,
        receipt.id
    );
    assert_eq!(snapshot(&c), after);
    let mut changed = input;
    changed.note = Some("Autre".into());
    assert!(record_repayment(&mut c, changed).is_err());
    assert_eq!(snapshot(&c), after);
    // Count the physical repayment in the account balances once, then close twice.
    let wave = fixture::id(&c, "Wave 1");
    let cash = fixture::id(&c, "Espèces");
    let mut balances = fixture::balances(&c);
    for b in &mut balances {
        if b.account_id == wave {
            b.amount -= 50_000;
        }
        if b.account_id == cash {
            b.amount += 35_000;
        }
    }
    let preview = db::preview_inventory(
        &c,
        InventoryPreviewInput {
            balances: balances.clone(),
        },
    )
    .unwrap();
    assert_eq!(preview.actual_total, 5_000_000);
    assert_eq!(preview.variance, 0);
    fixture::close(&mut c, balances.clone());
    let input = payment(&c, &client, 15_000, "2026-02-02");
    record_repayment(&mut c, input).unwrap();
    balances
        .iter_mut()
        .find(|b| b.account_id == cash)
        .unwrap()
        .amount += 15_000;
    fixture::close(&mut c, balances);
    assert_eq!(db::get_dashboard(&c).unwrap().expected_capital, 5_000_000);
    assert_eq!(customers(&c).unwrap()[0].remaining, 0);
}

#[test]
fn allocation_order_uses_issued_then_created_then_id_and_backdates_are_frozen() {
    let mut c = fixture::multi_database();
    let customer = client(&mut c, "Awa Fall");
    let template = loan(&mut c, &customer, 100, "2026-04-01");
    for (id, issued, created) in [
        ("b", "2026-01-01", "2026-03-01T10:00:00Z"),
        ("a", "2026-01-01", "2026-03-01T10:00:00Z"),
        ("c", "2026-01-01", "2026-02-01T10:00:00Z"),
    ] {
        c.execute("INSERT INTO debts(id,customer_id,customer_name,phone,provider,principal,remaining,issued_at,due_date,note,status,created_at,account_id,account_name,account_identifier) SELECT ?1,customer_id,customer_name,phone,provider,100,100,?2,NULL,NULL,'open',?3,account_id,account_name,account_identifier FROM debts WHERE id=?4",params![id,issued,created,template.id]).unwrap();
    }
    let input = payment(&c, &customer, 250, "2026-02-01");
    let receipt = record_repayment(&mut c, input).unwrap();
    assert_eq!(
        receipt
            .allocations
            .iter()
            .map(|a| a.debt_id.as_str())
            .collect::<Vec<_>>(),
        vec!["c", "a", "b"]
    );
    assert_eq!(receipt.allocations[2].remaining_after, Some(50));
    loan(&mut c, &customer, 500, "2025-01-01");
    assert_eq!(
        repayments(&c, None).unwrap()[0].allocations,
        receipt.allocations
    );
}

#[test]
fn stale_preview_rejects_and_transaction_failures_leave_no_partial_changes() {
    let mut c = fixture::multi_database();
    let customer = client(&mut c, "Awa Fall");
    loan(&mut c, &customer, 20_000, "2026-01-02");
    let stale = payment(&c, &customer, 10_000, "2026-02-01");
    loan(&mut c, &customer, 5_000, "2026-01-01");
    let before = snapshot(&c);
    assert!(record_repayment(&mut c, stale)
        .unwrap_err()
        .to_string()
        .contains("Actualisez"));
    assert_eq!(snapshot(&c), before);
    let input = payment(&c, &customer, 10_000, "2026-02-01");
    // Fail on the second allocation after the first payment, debt balance and receipt were written.
    c.execute_batch("CREATE TRIGGER fail_allocation BEFORE INSERT ON repayment_allocations WHEN NEW.position=1 BEGIN SELECT RAISE(ABORT,'simulated disk failure'); END;").unwrap();
    assert!(record_repayment(&mut c, input.clone()).is_err());
    assert_eq!(snapshot(&c), before);
    c.execute_batch("DROP TRIGGER fail_allocation; CREATE TRIGGER fail_audit BEFORE INSERT ON audit_events WHEN NEW.action='customer_repayment_recorded' BEGIN SELECT RAISE(ABORT,'simulated audit failure'); END;").unwrap();
    assert!(record_repayment(&mut c, input.clone()).is_err());
    assert_eq!(snapshot(&c), before);
    c.execute_batch("DROP TRIGGER fail_audit").unwrap();
    record_repayment(&mut c, input).unwrap();
}

#[test]
fn client_lifecycle_preserves_snapshots_and_creation_requests_are_idempotent() {
    let mut c = fixture::multi_database();
    let input = SaveDebtCustomerInput {
        request_id: request(),
        customer_id: None,
        name: "Awa Fall".into(),
        phone: "771234567".into(),
        active: true,
    };
    let customer = save_customer(&mut c, input.clone()).unwrap();
    assert_eq!(
        save_customer(&mut c, input.clone()).unwrap().id,
        customer.id
    );
    let mut edit = input;
    edit.customer_id = Some(customer.id.clone());
    assert!(save_customer(&mut c, edit.clone()).is_err());
    edit.request_id = request();
    edit.name = "Awa Ndiaye".into();
    let create = CreateClientDebtInput {
        request_id: request(),
        customer_id: customer.id.clone(),
        account_id: fixture::id(&c, "Orange 2"),
        amount: 100,
        issued_at: "2026-01-01".into(),
        due_date: None,
        note: None,
    };
    let debt = create_debt(&mut c, create.clone()).unwrap();
    assert_eq!(create_debt(&mut c, create.clone()).unwrap().id, debt.id);
    let mut changed = create.clone();
    changed.amount = 101;
    assert!(create_debt(&mut c, changed).is_err());
    let pay = payment(&c, &customer, 50, "2026-01-01");
    record_repayment(&mut c, pay).unwrap();
    save_customer(&mut c, edit.clone()).unwrap();
    assert_eq!(
        db::list_debts(&c, None).unwrap()[0].customer_name,
        "Awa Fall"
    );
    assert_eq!(repayments(&c, None).unwrap()[0].customer_name, "Awa Fall");
    edit.request_id = request();
    edit.active = false;
    assert!(save_customer(&mut c, edit.clone()).is_err());
    let pay = payment(&c, &customer, 50, "2026-01-01");
    record_repayment(&mut c, pay).unwrap();
    save_customer(&mut c, edit.clone()).unwrap();
    let mut create = create;
    create.request_id = request();
    assert!(create_debt(&mut c, create.clone()).is_err());
    edit.request_id = request();
    edit.active = true;
    save_customer(&mut c, edit).unwrap();
    assert_eq!(
        create_debt(&mut c, create).unwrap().customer_name,
        "Awa Ndiaye"
    );
    assert!(crate::custody::customers(&c).unwrap().is_empty());
}

#[test]
fn validates_limits_accounts_dates_and_other_customers_without_cross_allocation() {
    let mut c = fixture::multi_database();
    let a = client(&mut c, "Awa Fall");
    let b = client(&mut c, "Binta Fall");
    loan(&mut c, &a, 10_000, "2026-01-01");
    loan(&mut c, &b, 50_000, "2026-01-01");
    for amount in [0, -1, 10_001, i64::MAX] {
        assert!(preview(
            &c,
            &RepaymentPreviewInput {
                customer_id: a.id.clone(),
                amount,
                paid_at: "2026-01-01".into()
            }
        )
        .is_err());
    }
    for date in ["2025-12-31", "2026-02-30", "2026-1-1"] {
        assert!(preview(
            &c,
            &RepaymentPreviewInput {
                customer_id: a.id.clone(),
                amount: 1,
                paid_at: date.into()
            }
        )
        .is_err());
    }
    let mut pay = payment(&c, &a, 1_000, "2026-01-01");
    pay.account_id = fixture::id(&c, "Wave 2");
    c.execute(
        "UPDATE accounts SET active=0 WHERE id=?1",
        [&pay.account_id],
    )
    .unwrap();
    assert!(record_repayment(&mut c, pay).is_err());
    for name in ["Orange 1", "Wave 1", "Djamo 1", "Espèces"] {
        let mut pay = payment(&c, &a, 2_500, "2026-01-01");
        pay.account_id = fixture::id(&c, name);
        record_repayment(&mut c, pay).unwrap();
    }
    assert_eq!(customer(&c, &b.id, false).unwrap().remaining, 50_000);
    assert_eq!(customer(&c, &a.id, false).unwrap().remaining, 0);
    let oversized = CreateClientDebtInput {
        request_id: request(),
        customer_id: a.id,
        account_id: fixture::id(&c, "Wave 1"),
        amount: 9_007_199_254_740_991,
        issued_at: "2026-01-01".into(),
        due_date: None,
        note: None,
    };
    assert!(create_debt(&mut c, oversized).is_err());
    assert!(serde_json::from_value::<RepaymentPreviewInput>(
        json!({"customerId":b.id,"amount":1.5,"paidAt":"2026-01-01"})
    )
    .is_err());
}

#[test]
fn migration_groups_only_normalized_pairs_and_preserves_legacy_payments() {
    let c = Connection::open_in_memory().unwrap();
    fixture::legacy_database(&c);
    for (id, name, phone) in [
        ("same", "  AWA   Ndiaye ", "77 (123)-45 67"),
        ("other-name", "Awa Fall", "771234567"),
        ("prefix", "Awa Ndiaye", "+221771234567"),
    ] {
        c.execute("INSERT INTO debts SELECT ?1,?2,?3,provider,principal,remaining,issued_at,due_date,note,status,cancellation_reason,created_at FROM debts WHERE id='debt'",params![id,name,phone]).unwrap();
    }
    let historical = fixture::historical_data(&c);
    db::migrate(&c).unwrap();
    assert_eq!(db::schema_version(&c).unwrap(), 5);
    assert_eq!(fixture::historical_data(&c), historical);
    assert_eq!(customers(&c).unwrap().len(), 3);
    let debts = db::list_debts(&c, None).unwrap();
    assert_eq!(
        debts.iter().find(|d| d.id == "same").unwrap().customer_id,
        debts.iter().find(|d| d.id == "debt").unwrap().customer_id
    );
    let receipts = repayments(&c, None).unwrap();
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0].legacy);
    assert_eq!(receipts[0].id, "payment");
    assert_eq!(receipts[0].amount, 20_000);
    assert_eq!(receipts[0].allocations[0].remaining_after, None);
    let after = snapshot(&c);
    db::migrate(&c).unwrap();
    assert_eq!(snapshot(&c), after);
}

#[test]
fn repayments_filter_on_payment_date_and_new_history_is_immutable() {
    let mut c = fixture::multi_database();
    let client = client(&mut c, "Awa Fall");
    loan(&mut c, &client, 20_000, "2026-01-01");
    loan(&mut c, &client, 30_000, "2026-01-02");
    let input = payment(&c, &client, 35_000, "2026-02-01");
    record_repayment(&mut c, input).unwrap();
    let report = db::get_report(
        &c,
        ReportFilters {
            from: Some("2026-02-01".into()),
            to: Some("2026-02-28".into()),
        },
    )
    .unwrap();
    assert!(report.debts.is_empty());
    assert_eq!(report.repayments.len(), 1);
    assert_eq!(report.repayments[0].allocations.len(), 2);
    assert_eq!(report.total_positive, 0);
    for table in [
        "customer_repayments",
        "repayment_allocations",
        "debt_payments",
        "debt_requests",
    ] {
        assert!(c.execute_batch(&format!("DELETE FROM {table}")).is_err());
    }
    assert!(c
        .execute_batch("UPDATE customer_repayments SET amount=1")
        .is_err());
    assert!(c
        .execute_batch("UPDATE repayment_allocations SET amount=1")
        .is_err());
}
