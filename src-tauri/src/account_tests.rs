use super::{test_support::*, *};

#[test]
fn tauri_payloads_use_stable_account_ids_and_reject_fractional_or_null_balances() {
    let db = multi_database();
    let dashboard = serde_json::to_value(get_dashboard(&db).unwrap()).unwrap();
    let account = &dashboard["accounts"][0];
    assert!(account["accountId"].is_string());
    assert!(account["lastBalance"].is_i64());
    assert!(account.get("snapshot").is_none());
    let detail = &dashboard["lastInventory"]["accountBalances"][0];
    assert!(detail["accountId"].is_string());
    assert!(detail["previousAmount"].is_null());
    assert!(detail["delta"].is_null());
    assert_eq!(
        dashboard["lastInventory"]["balances"]["orangeMoney"],
        1_500_000
    );
    let value = serde_json::json!({"balances":[{"accountId":"sim-id","amount":1234}]});
    let parsed: InventoryPreviewInput = serde_json::from_value(value).unwrap();
    assert_eq!(parsed.balances[0].account_id, "sim-id");
    for amount in [
        serde_json::json!(1.5),
        serde_json::Value::Null,
        serde_json::json!("10"),
    ] {
        assert!(serde_json::from_value::<InventoryPreviewInput>(
            serde_json::json!({"balances":[{"accountId":"sim-id","amount":amount}]})
        )
        .is_err());
    }
}

#[test]
fn multiple_sims_opening_and_reallocation_preserve_capital() {
    let mut db = multi_database();
    let opening = last_inventory(&db).unwrap();
    assert_eq!(opening.account_balances.len(), 6);
    assert_eq!(opening.balances.orange_money, 1_500_000);
    assert_eq!(opening.balances.wave, 1_200_000);
    assert_eq!(opening.actual_total, 5_000_000);
    assert!(opening.account_balances.iter().all(|b| b.delta.is_none()));
    let om1 = id(&db, "Orange 1");
    let om2 = id(&db, "Orange 2");
    let mut inputs = balances(&db);
    inputs
        .iter_mut()
        .find(|b| b.account_id == om1)
        .unwrap()
        .amount -= 200_000;
    inputs
        .iter_mut()
        .find(|b| b.account_id == om2)
        .unwrap()
        .amount += 200_000;
    let next = close(&mut db, inputs);
    assert_eq!(next.actual_total, 5_000_000);
    assert_eq!(next.variance, 0);
    assert_eq!(next.balances.orange_money, 1_500_000);
    assert_eq!(
        next.account_balances
            .iter()
            .find(|b| b.account.account_id == om1)
            .unwrap()
            .delta,
        Some(-200_000)
    );
}

#[test]
fn opening_rejects_invalid_capital_names_and_extra_cash() {
    let mut input = multi_setup();
    input.accounts[0].amount -= 1;
    assert!(accounts::validate_opening(&input).is_err());
    input.accounts[0].amount = -1;
    assert!(accounts::validate_opening(&input).is_err());
    let mut input = multi_setup();
    input.accounts[1].name = " orange 1 ".into();
    assert!(accounts::validate_opening(&input).is_err());
    let mut input = multi_setup();
    input.accounts[1].provider = "cash".into();
    assert!(accounts::validate_opening(&input).is_err());
    let mut input = multi_setup();
    input.accounts[0].name = " ".into();
    assert!(accounts::validate_opening(&input).is_err());
}

#[test]
fn inventory_rejects_missing_duplicate_negative_unknown_and_archived_accounts() {
    let mut db = multi_database();
    let valid = balances(&db);
    let mut missing = valid.clone();
    missing.pop();
    let mut duplicate = valid.clone();
    duplicate[1] = duplicate[0].clone();
    let mut negative = valid.clone();
    negative[0].amount = -1;
    let mut unknown = valid.clone();
    unknown[0].account_id = "missing".into();
    let mut too_large = valid.clone();
    too_large[0].amount = 9_007_199_254_740_992;
    for inputs in [missing, duplicate, negative, unknown, too_large] {
        assert!(preview_inventory(
            &db,
            InventoryPreviewInput {
                balances: inputs.clone()
            }
        )
        .is_err());
        assert!(close_inventory(
            &mut db,
            CloseInventoryInput {
                balances: inputs,
                variance_category: None,
                variance_note: None
            }
        )
        .is_err());
    }
    assert_eq!(list_inventories(&db, None).unwrap().len(), 1);
    let extra = accounts::create(
        &mut db,
        CreateAccountInput {
            provider: "wave".into(),
            name: "Réserve".into(),
            identifier: None,
        },
    )
    .unwrap();
    assert_eq!(extra.last_balance, None);
    let pending = balances(&db);
    let preview = preview_inventory(
        &db,
        InventoryPreviewInput {
            balances: pending.clone(),
        },
    )
    .unwrap();
    assert!(preview
        .account_balances
        .iter()
        .find(|b| b.account.account_id == extra.snapshot.account_id)
        .unwrap()
        .delta
        .is_none());
    accounts::set_active(&mut db, &extra.snapshot.account_id, false).unwrap();
    // Valid at preview time, stale at closure: active account set must be rechecked.
    assert!(close_inventory(
        &mut db,
        CloseInventoryInput {
            balances: pending,
            variance_category: None,
            variance_note: None
        }
    )
    .is_err());
    let mut archived = valid;
    archived[0].account_id = extra.snapshot.account_id;
    assert!(accounts::preview(&db, &archived).is_err());
}

#[test]
fn accounts_require_unique_names_and_keep_historical_snapshots() {
    let mut db = multi_database();
    let account_id = id(&db, "Orange 1");
    let original = journal(&mut db, &account_id);
    let capital = get_dashboard(&db).unwrap().expected_capital;
    let renamed = accounts::update(
        &mut db,
        UpdateAccountInput {
            account_id: account_id.clone(),
            name: "Boutique façade".into(),
            identifier: Some("771112233".into()),
        },
    )
    .unwrap();
    assert_eq!(renamed.snapshot.account_id, account_id);
    assert_eq!(get_dashboard(&db).unwrap().expected_capital, capital);
    let original_again = list_journal_entries(&db, None).unwrap().remove(0);
    assert_eq!(
        original_again.account_snapshot.name,
        original.account_snapshot.name
    );
    assert_eq!(
        original_again.account_snapshot.identifier,
        original.account_snapshot.identifier
    );
    assert!(last_inventory(&db)
        .unwrap()
        .account_balances
        .iter()
        .any(|b| b.account.name == "Orange 1"));
    assert!(accounts::create(
        &mut db,
        CreateAccountInput {
            provider: "orange_money".into(),
            name: " boutique FAÇADE ".into(),
            identifier: None
        }
    )
    .is_err());
    let new = accounts::create(
        &mut db,
        CreateAccountInput {
            provider: "wave".into(),
            name: "Boutique façade".into(),
            identifier: None,
        },
    )
    .unwrap();
    assert!(new.last_balance.is_none());
    assert_eq!(get_dashboard(&db).unwrap().expected_capital, capital);
    assert!(db
        .execute(
            "UPDATE accounts SET provider='wave' WHERE id=?1",
            [&account_id]
        )
        .is_err());
    assert!(db
        .execute("DELETE FROM accounts WHERE id=?1", [&account_id])
        .is_err());
    assert!(accounts::create(
        &mut db,
        CreateAccountInput {
            provider: "cash".into(),
            name: "Caisse 2".into(),
            identifier: None
        }
    )
    .is_err());
}

#[test]
fn archive_requires_zero_and_no_later_operations_reversals_keep_original_account() {
    let mut db = multi_database();
    let om = id(&db, "Orange 1");
    let cash = id(&db, "Espèces");
    assert!(accounts::set_active(&mut db, &cash, false).is_err());
    assert!(accounts::set_active(&mut db, &om, false).is_err());
    let mut inputs = balances(&db);
    inputs
        .iter_mut()
        .find(|b| b.account_id == om)
        .unwrap()
        .amount = 0;
    close(&mut db, inputs);
    let entry = journal(&mut db, &om);
    assert!(accounts::set_active(&mut db, &om, false).is_err());
    let inputs = balances(&db);
    close(&mut db, inputs);
    accounts::set_active(&mut db, &om, false).unwrap();
    assert!(accounts::get(&db, &om, true).is_err());
    assert!(create_journal_entry(
        &mut db,
        CreateJournalEntryInput {
            entry_type: "sale".into(),
            amount: 1,
            account_id: om.clone(),
            occurred_at: "2026-09-01".into(),
            reference: None,
            note: None
        }
    )
    .is_err());
    accounts::update(
        &mut db,
        UpdateAccountInput {
            account_id: om.clone(),
            name: "Ancienne SIM".into(),
            identifier: None,
        },
    )
    .unwrap();
    let reversal = reverse_journal_entry(
        &mut db,
        ReverseEntryInput {
            entry_id: entry.id.clone(),
            reason: "Annulation de la recette".into(),
        },
    )
    .unwrap();
    assert_eq!(reversal.account_snapshot.account_id, om);
    assert_eq!(reversal.account_snapshot.name, "Orange 1");
    assert_eq!(reversal.signed_amount, -entry.signed_amount);
    assert!(reverse_journal_entry(
        &mut db,
        ReverseEntryInput {
            entry_id: entry.id,
            reason: "Deuxième tentative".into()
        }
    )
    .is_err());
    assert!(accounts::set_active(&mut db, &om, true).unwrap().active);
    assert!(list_audit_events(&db, 500)
        .unwrap()
        .iter()
        .any(|e| e.action == "account_archived"));
}

#[test]
fn djamo_debt_can_be_repaid_on_other_sim_and_cash_after_source_archive() {
    let mut db = multi_database();
    let djamo = id(&db, "Djamo 1");
    let wave = id(&db, "Wave 2");
    let cash = id(&db, "Espèces");
    let loan = debt(&mut db, &djamo);
    assert_eq!(loan.provider, "djamo");
    let mut inputs = balances(&db);
    inputs
        .iter_mut()
        .find(|b| b.account_id == djamo)
        .unwrap()
        .amount -= 50_000;
    let inventory = close(&mut db, inputs);
    assert_eq!(inventory.liquidity, 4_950_000);
    assert_eq!(inventory.receivables, 50_000);
    assert_eq!(inventory.actual_total, 5_000_000);
    let partial = payment(&mut db, &loan.id, &wave, 20_000);
    assert_eq!(partial.remaining, 30_000);
    let mut inputs = balances(&db);
    inputs
        .iter_mut()
        .find(|b| b.account_id == wave)
        .unwrap()
        .amount += 20_000;
    let inventory = close(&mut db, inputs);
    assert_eq!(inventory.actual_total, 5_000_000);
    assert_eq!(inventory.variance, 0);
    // Move remaining Djamo liquidity into cash and archive the empty source.
    let mut inputs = balances(&db);
    inputs
        .iter_mut()
        .find(|b| b.account_id == djamo)
        .unwrap()
        .amount = 0;
    inputs
        .iter_mut()
        .find(|b| b.account_id == cash)
        .unwrap()
        .amount += 750_000;
    close(&mut db, inputs);
    accounts::set_active(&mut db, &djamo, false).unwrap();
    let paid = payment(&mut db, &loan.id, &cash, 30_000);
    assert_eq!(paid.status, "paid");
    assert_eq!(paid.payments.len(), 2);
    accounts::update(
        &mut db,
        UpdateAccountInput {
            account_id: wave.clone(),
            name: "Wave renommée".into(),
            identifier: None,
        },
    )
    .unwrap();
    let paid = list_debts(&db, None).unwrap().remove(0);
    assert!(paid
        .payments
        .iter()
        .any(|p| p.account_snapshot.name == "Wave 2"));
    assert!(!accounts::get(&db, &djamo, false).unwrap().active);
}

#[test]
fn v1_migration_is_atomic_preserves_all_original_columns_and_runs_once() {
    let db = Connection::open_in_memory().unwrap();
    legacy_database(&db);
    let before = historical_data(&db);
    migrate(&db).unwrap();
    assert_eq!(schema_version(&db).unwrap(), crate::db::SCHEMA_VERSION);
    assert_eq!(historical_data(&db), before);
    assert_eq!(accounts::list(&db).unwrap().len(), 4);
    let inventories = list_inventories(&db, None).unwrap();
    assert!(inventories
        .iter()
        .all(|i| i.account_balances.len() == 4 && i.account_balances.iter().all(|b| b.legacy)));
    assert_eq!(
        list_debts(&db, None).unwrap()[0]
            .account_snapshot
            .account_id,
        "legacy-wave"
    );
    assert_eq!(
        list_debts(&db, None).unwrap()[0].payments[0]
            .account_snapshot
            .account_id,
        "legacy-cash"
    );
    migrate(&db).unwrap();
    assert_eq!(accounts::list(&db).unwrap().len(), 4);
    assert_eq!(historical_data(&db), before);
    assert_eq!(
        list_audit_events(&db, 500)
            .unwrap()
            .iter()
            .filter(|e| e.action == "schema_migrated")
            .count(),
        1
    );
    assert!(!db
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    for sql in [
        "UPDATE inventories SET cash=0",
        "DELETE FROM inventories",
        "UPDATE inventory_account_balances SET amount=0",
        "DELETE FROM inventory_account_balances",
        "UPDATE journal_entries SET amount=1",
        "DELETE FROM audit_events",
    ] {
        assert!(db.execute(sql, []).is_err(), "{sql}");
    }
}

#[test]
fn broken_v1_migration_rolls_back_and_restores_foreign_keys() {
    let db = Connection::open_in_memory().unwrap();
    legacy_database(&db);
    db.execute(
        "UPDATE journal_entries SET payment_account='unknown' WHERE id='sale'",
        [],
    )
    .unwrap();
    let before = historical_data(&db);
    assert!(migrate(&db).is_err());
    assert_eq!(schema_version(&db).unwrap(), 1);
    assert_eq!(historical_data(&db), before);
    assert!(!db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='accounts')",
            [],
            |r| r.get::<_, bool>(0)
        )
        .unwrap());
    assert!(db
        .query_row("PRAGMA foreign_keys", [], |r| r.get::<_, bool>(0))
        .unwrap());
}
