use std::path::Path;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::{
        clean_optional, clean_required, signed_journal_amount, validate_balances,
        validate_debt_provider, validate_payment_account, validate_variance_explanation,
    },
    error::{AppError, AppResult},
    models::*,
};

pub const SCHEMA_VERSION: i64 = 1;

pub fn open_database(path: &Path, database_key: &[u8]) -> AppResult<Connection> {
    let connection = Connection::open(path)?;
    let key_hex = hex::encode(database_key);
    connection.execute_batch(&format!(
        "PRAGMA key = \"x'{key_hex}'\";
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;"
    ))?;
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|_| AppError::InvalidPin)?;
    Ok(connection)
}

pub fn migrate(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS business_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            business_name TEXT NOT NULL,
            currency TEXT NOT NULL DEFAULT 'XOF',
            timezone TEXT NOT NULL DEFAULT 'Africa/Dakar',
            inventory_interval_minutes INTEGER NOT NULL DEFAULT 240 CHECK (inventory_interval_minutes BETWEEN 15 AND 10080),
            auto_lock_minutes INTEGER NOT NULL DEFAULT 15 CHECK (auto_lock_minutes BETWEEN 1 AND 240),
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS inventories (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL CHECK (kind IN ('opening', 'regular')),
            closed_at TEXT NOT NULL,
            orange_money INTEGER NOT NULL CHECK (orange_money >= 0),
            wave INTEGER NOT NULL CHECK (wave >= 0),
            djamo INTEGER NOT NULL CHECK (djamo >= 0),
            cash INTEGER NOT NULL CHECK (cash >= 0),
            receivables INTEGER NOT NULL CHECK (receivables >= 0),
            liquidity INTEGER NOT NULL CHECK (liquidity >= 0),
            expected_total INTEGER NOT NULL,
            actual_total INTEGER NOT NULL,
            variance INTEGER NOT NULL,
            variance_category TEXT,
            variance_note TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_inventories_closed_at ON inventories(closed_at);

        CREATE TABLE IF NOT EXISTS journal_entries (
            id TEXT PRIMARY KEY,
            entry_type TEXT NOT NULL,
            amount INTEGER NOT NULL CHECK (amount > 0),
            signed_amount INTEGER NOT NULL,
            payment_account TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            posted_at TEXT NOT NULL,
            reference TEXT,
            note TEXT,
            reverses_id TEXT REFERENCES journal_entries(id),
            UNIQUE(reverses_id)
        );
        CREATE INDEX IF NOT EXISTS idx_journal_posted_at ON journal_entries(posted_at);
        CREATE INDEX IF NOT EXISTS idx_journal_occurred_at ON journal_entries(occurred_at);

        CREATE TABLE IF NOT EXISTS debts (
            id TEXT PRIMARY KEY,
            customer_name TEXT NOT NULL,
            phone TEXT NOT NULL,
            provider TEXT NOT NULL CHECK (provider IN ('orange_money', 'wave')),
            principal INTEGER NOT NULL CHECK (principal > 0),
            remaining INTEGER NOT NULL CHECK (remaining >= 0),
            issued_at TEXT NOT NULL,
            due_date TEXT,
            note TEXT,
            status TEXT NOT NULL CHECK (status IN ('open', 'partial', 'paid', 'cancelled')),
            cancellation_reason TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_debts_phone ON debts(phone);
        CREATE INDEX IF NOT EXISTS idx_debts_status ON debts(status);

        CREATE TABLE IF NOT EXISTS debt_payments (
            id TEXT PRIMARY KEY,
            debt_id TEXT NOT NULL REFERENCES debts(id),
            amount INTEGER NOT NULL CHECK (amount > 0),
            account TEXT NOT NULL,
            paid_at TEXT NOT NULL,
            note TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_debt_payments_debt_id ON debt_payments(debt_id);

        CREATE TABLE IF NOT EXISTS audit_events (
            id TEXT PRIMARY KEY,
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            details_json TEXT NOT NULL,
            occurred_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_audit_occurred_at ON audit_events(occurred_at);

        PRAGMA user_version = 1;",
    )?;
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn validate_date(value: &str, label: &str) -> AppResult<String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("{label} doit être une date valide.")))?;
    Ok(value.to_string())
}

fn audit_tx(
    tx: &Transaction<'_>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    details: serde_json::Value,
) -> AppResult<()> {
    tx.execute(
        "INSERT INTO audit_events (id, action, entity_type, entity_id, details_json, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            Uuid::new_v4().to_string(),
            action,
            entity_type,
            entity_id,
            details.to_string(),
            now()
        ],
    )?;
    Ok(())
}

pub fn initialize_business(
    connection: &mut Connection,
    input: &SetupInput,
    balances: &AccountBalances,
) -> AppResult<()> {
    let business_name = clean_required(&input.business_name, "Le nom de la boutique", 2)?;
    let liquidity = validate_balances(balances)?;
    let timestamp = now();
    let inventory_id = Uuid::new_v4().to_string();
    let tx = connection.transaction()?;
    tx.execute(
        "INSERT INTO business_settings
         (id, business_name, currency, timezone, inventory_interval_minutes, auto_lock_minutes, created_at)
         VALUES (1, ?1, 'XOF', 'Africa/Dakar', 240, 15, ?2)",
        params![business_name, timestamp],
    )?;
    tx.execute(
        "INSERT INTO inventories
         (id, kind, closed_at, orange_money, wave, djamo, cash, receivables, liquidity,
          expected_total, actual_total, variance, variance_category, variance_note)
         VALUES (?1, 'opening', ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7, ?7, 0, NULL, NULL)",
        params![
            inventory_id,
            timestamp,
            balances.orange_money,
            balances.wave,
            balances.djamo,
            balances.cash,
            liquidity
        ],
    )?;
    audit_tx(
        &tx,
        "business_initialized",
        "business",
        Some("1"),
        json!({
            "initialCapital": liquidity,
            "orangeMoney": balances.orange_money,
            "wave": balances.wave,
            "djamo": balances.djamo,
            "cash": balances.cash
        }),
    )?;
    tx.commit()?;
    Ok(())
}

pub fn get_settings(connection: &Connection) -> AppResult<BusinessSettings> {
    connection
        .query_row(
            "SELECT business_name, currency, timezone, inventory_interval_minutes, auto_lock_minutes, created_at
             FROM business_settings WHERE id = 1",
            [],
            |row| {
                Ok(BusinessSettings {
                    business_name: row.get(0)?,
                    currency: row.get(1)?,
                    timezone: row.get(2)?,
                    inventory_interval_minutes: row.get(3)?,
                    auto_lock_minutes: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .map_err(AppError::from)
}

pub fn update_settings(
    connection: &mut Connection,
    input: UpdateSettingsInput,
) -> AppResult<BusinessSettings> {
    let name = clean_required(&input.business_name, "Le nom de la boutique", 2)?;
    if !(15..=10_080).contains(&input.inventory_interval_minutes) {
        return Err(AppError::Validation(
            "L’intervalle d’inventaire doit être compris entre 15 minutes et 7 jours.".into(),
        ));
    }
    if !(1..=240).contains(&input.auto_lock_minutes) {
        return Err(AppError::Validation(
            "Le verrouillage doit être compris entre 1 et 240 minutes.".into(),
        ));
    }
    let tx = connection.transaction()?;
    tx.execute(
        "UPDATE business_settings
         SET business_name = ?1, inventory_interval_minutes = ?2, auto_lock_minutes = ?3
         WHERE id = 1",
        params![
            name,
            input.inventory_interval_minutes,
            input.auto_lock_minutes
        ],
    )?;
    audit_tx(
        &tx,
        "settings_updated",
        "business",
        Some("1"),
        json!({
            "businessName": name,
            "inventoryIntervalMinutes": input.inventory_interval_minutes,
            "autoLockMinutes": input.auto_lock_minutes
        }),
    )?;
    tx.commit()?;
    get_settings(connection)
}

#[derive(Clone)]
struct InventoryRow {
    id: String,
    kind: String,
    closed_at: String,
    balances: AccountBalances,
    receivables: Money,
    liquidity: Money,
    expected_total: Money,
    actual_total: Money,
    variance: Money,
    variance_category: Option<String>,
    variance_note: Option<String>,
}

fn map_inventory_row(row: &Row<'_>) -> rusqlite::Result<InventoryRow> {
    Ok(InventoryRow {
        id: row.get(0)?,
        kind: row.get(1)?,
        closed_at: row.get(2)?,
        balances: AccountBalances {
            orange_money: row.get(3)?,
            wave: row.get(4)?,
            djamo: row.get(5)?,
            cash: row.get(6)?,
        },
        receivables: row.get(7)?,
        liquidity: row.get(8)?,
        expected_total: row.get(9)?,
        actual_total: row.get(10)?,
        variance: row.get(11)?,
        variance_category: row.get(12)?,
        variance_note: row.get(13)?,
    })
}

fn load_inventory_rows(connection: &Connection) -> AppResult<Vec<InventoryRow>> {
    let mut statement = connection.prepare(
        "SELECT id, kind, closed_at, orange_money, wave, djamo, cash, receivables,
                liquidity, expected_total, actual_total, variance, variance_category, variance_note
         FROM inventories ORDER BY closed_at ASC",
    )?;
    let rows = statement
        .query_map([], map_inventory_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn to_inventory(row: &InventoryRow, previous: Option<&InventoryRow>) -> Inventory {
    let previous_balances = previous
        .map(|v| v.balances.clone())
        .unwrap_or_else(|| row.balances.clone());
    Inventory {
        id: row.id.clone(),
        kind: row.kind.clone(),
        closed_at: row.closed_at.clone(),
        balances: row.balances.clone(),
        receivables: row.receivables,
        liquidity: row.liquidity,
        expected_total: row.expected_total,
        actual_total: row.actual_total,
        variance: row.variance,
        variance_category: row.variance_category.clone(),
        variance_note: row.variance_note.clone(),
        delta: AccountBalances {
            orange_money: row.balances.orange_money - previous_balances.orange_money,
            wave: row.balances.wave - previous_balances.wave,
            djamo: row.balances.djamo - previous_balances.djamo,
            cash: row.balances.cash - previous_balances.cash,
        },
    }
}

fn date_in_filter(timestamp: &str, filters: &ReportFilters) -> bool {
    let date = timestamp.get(..10).unwrap_or(timestamp);
    filters.from.as_deref().is_none_or(|from| date >= from)
        && filters.to.as_deref().is_none_or(|to| date <= to)
}

pub fn list_inventories(
    connection: &Connection,
    filters: Option<&ReportFilters>,
) -> AppResult<Vec<Inventory>> {
    let rows = load_inventory_rows(connection)?;
    let mut items: Vec<Inventory> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| to_inventory(row, index.checked_sub(1).map(|i| &rows[i])))
        .filter(|item| filters.is_none_or(|f| date_in_filter(&item.closed_at, f)))
        .collect();
    items.reverse();
    Ok(items)
}

fn last_inventory_row(connection: &Connection) -> AppResult<InventoryRow> {
    connection
        .query_row(
            "SELECT id, kind, closed_at, orange_money, wave, djamo, cash, receivables,
                    liquidity, expected_total, actual_total, variance, variance_category, variance_note
             FROM inventories ORDER BY closed_at DESC LIMIT 1",
            [],
            map_inventory_row,
        )
        .map_err(AppError::from)
}

pub fn last_inventory(connection: &Connection) -> AppResult<Inventory> {
    let all = load_inventory_rows(connection)?;
    let last = all.last().ok_or(AppError::NotFound)?;
    let previous = all.len().checked_sub(2).map(|index| &all[index]);
    Ok(to_inventory(last, previous))
}

pub fn open_receivables(connection: &Connection) -> AppResult<Money> {
    connection
        .query_row(
            "SELECT COALESCE(SUM(remaining), 0) FROM debts WHERE status IN ('open', 'partial')",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)
}

fn journal_net_after(connection: &Connection, timestamp: &str) -> AppResult<Money> {
    connection
        .query_row(
            "SELECT COALESCE(SUM(signed_amount), 0) FROM journal_entries WHERE posted_at > ?1",
            [timestamp],
            |row| row.get(0),
        )
        .map_err(AppError::from)
}

pub fn preview_inventory(
    connection: &Connection,
    input: InventoryPreviewInput,
) -> AppResult<InventoryPreview> {
    let balances = AccountBalances {
        orange_money: input.orange_money,
        wave: input.wave,
        djamo: input.djamo,
        cash: input.cash,
    };
    let liquidity = validate_balances(&balances)?;
    let previous = last_inventory_row(connection)?;
    let receivables = open_receivables(connection)?;
    let expected_total = previous
        .actual_total
        .checked_add(journal_net_after(connection, &previous.closed_at)?)
        .ok_or_else(|| {
            AppError::Validation("Le capital attendu dépasse la limite autorisée.".into())
        })?;
    let actual_total = liquidity.checked_add(receivables).ok_or_else(|| {
        AppError::Validation("Le capital réel dépasse la limite autorisée.".into())
    })?;
    Ok(InventoryPreview {
        balances: balances.clone(),
        previous_balances: previous.balances.clone(),
        delta: AccountBalances {
            orange_money: balances.orange_money - previous.balances.orange_money,
            wave: balances.wave - previous.balances.wave,
            djamo: balances.djamo - previous.balances.djamo,
            cash: balances.cash - previous.balances.cash,
        },
        receivables,
        liquidity,
        expected_total,
        actual_total,
        variance: actual_total - expected_total,
    })
}

pub fn close_inventory(
    connection: &mut Connection,
    input: CloseInventoryInput,
) -> AppResult<Inventory> {
    let preview = preview_inventory(
        connection,
        InventoryPreviewInput {
            orange_money: input.orange_money,
            wave: input.wave,
            djamo: input.djamo,
            cash: input.cash,
        },
    )?;
    validate_variance_explanation(
        preview.variance,
        input.variance_category.as_deref(),
        input.variance_note.as_deref(),
    )?;
    let category = if preview.variance == 0 {
        None
    } else {
        clean_optional(input.variance_category)
    };
    let note = if preview.variance == 0 {
        None
    } else {
        clean_optional(input.variance_note)
    };
    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    let tx = connection.transaction()?;
    tx.execute(
        "INSERT INTO inventories
         (id, kind, closed_at, orange_money, wave, djamo, cash, receivables, liquidity,
          expected_total, actual_total, variance, variance_category, variance_note)
         VALUES (?1, 'regular', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id,
            timestamp,
            preview.balances.orange_money,
            preview.balances.wave,
            preview.balances.djamo,
            preview.balances.cash,
            preview.receivables,
            preview.liquidity,
            preview.expected_total,
            preview.actual_total,
            preview.variance,
            category,
            note
        ],
    )?;
    audit_tx(
        &tx,
        "inventory_closed",
        "inventory",
        Some(&id),
        json!({
            "expectedTotal": preview.expected_total,
            "actualTotal": preview.actual_total,
            "variance": preview.variance,
            "receivables": preview.receivables,
            "category": category
        }),
    )?;
    tx.commit()?;
    last_inventory(connection)
}

pub fn create_inventory_correction(
    connection: &mut Connection,
    input: InventoryCorrectionInput,
) -> AppResult<JournalEntry> {
    if input.amount <= 0 {
        return Err(AppError::Validation(
            "Le montant de correction doit être supérieur à zéro.".into(),
        ));
    }
    validate_payment_account(&input.payment_account)?;
    let reason = clean_required(&input.reason, "Le motif de correction", 3)?;
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM inventories WHERE id = ?1)",
        [&input.inventory_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound);
    }
    let signed_amount = match input.direction.as_str() {
        "increase" => input.amount,
        "decrease" => input
            .amount
            .checked_neg()
            .ok_or_else(|| AppError::Validation("Montant invalide.".into()))?,
        _ => {
            return Err(AppError::Validation(
                "Sens de correction non reconnu.".into(),
            ))
        }
    };
    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    let occurred_at = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    let reference = format!("inventory:{}", input.inventory_id);
    let tx = connection.transaction()?;
    tx.execute(
        "INSERT INTO journal_entries
         (id, entry_type, amount, signed_amount, payment_account, occurred_at, posted_at, reference, note, reverses_id)
         VALUES (?1, 'inventory_correction', ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
        params![
            id,
            input.amount,
            signed_amount,
            input.payment_account,
            occurred_at,
            timestamp,
            reference,
            reason
        ],
    )?;
    audit_tx(
        &tx,
        "inventory_corrected",
        "inventory",
        Some(&input.inventory_id),
        json!({
            "journalEntryId": id,
            "amount": input.amount,
            "direction": input.direction,
            "reason": reason
        }),
    )?;
    tx.commit()?;
    connection
        .query_row(
            "SELECT j.id, j.entry_type, j.amount, j.signed_amount, j.payment_account,
                    j.occurred_at, j.posted_at, j.reference, j.note, j.reverses_id, 0
             FROM journal_entries j WHERE j.id = ?1",
            [&id],
            map_journal_row,
        )
        .map_err(AppError::from)
}

fn map_journal_row(row: &Row<'_>) -> rusqlite::Result<JournalEntry> {
    Ok(JournalEntry {
        id: row.get(0)?,
        entry_type: row.get(1)?,
        amount: row.get(2)?,
        signed_amount: row.get(3)?,
        payment_account: row.get(4)?,
        occurred_at: row.get(5)?,
        posted_at: row.get(6)?,
        reference: row.get(7)?,
        note: row.get(8)?,
        reverses_id: row.get(9)?,
        reversed: row.get::<_, i64>(10)? != 0,
    })
}

pub fn list_journal_entries(
    connection: &Connection,
    filters: Option<&ReportFilters>,
) -> AppResult<Vec<JournalEntry>> {
    let mut statement = connection.prepare(
        "SELECT j.id, j.entry_type, j.amount, j.signed_amount, j.payment_account,
                j.occurred_at, j.posted_at, j.reference, j.note, j.reverses_id,
                EXISTS(SELECT 1 FROM journal_entries r WHERE r.reverses_id = j.id) AS reversed
         FROM journal_entries j ORDER BY j.occurred_at DESC, j.posted_at DESC",
    )?;
    let entries = statement
        .query_map([], map_journal_row)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|entry| filters.is_none_or(|f| date_in_filter(&entry.occurred_at, f)))
        .collect();
    Ok(entries)
}

pub fn create_journal_entry(
    connection: &mut Connection,
    input: CreateJournalEntryInput,
) -> AppResult<JournalEntry> {
    let signed_amount = signed_journal_amount(&input.entry_type, input.amount)?;
    validate_payment_account(&input.payment_account)?;
    let occurred_at = validate_date(&input.occurred_at, "La date")?;
    let reference = clean_optional(input.reference);
    let note = clean_optional(input.note);
    let id = Uuid::new_v4().to_string();
    let posted_at = now();
    let tx = connection.transaction()?;
    tx.execute(
        "INSERT INTO journal_entries
         (id, entry_type, amount, signed_amount, payment_account, occurred_at, posted_at, reference, note, reverses_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL)",
        params![
            id,
            input.entry_type,
            input.amount,
            signed_amount,
            input.payment_account,
            occurred_at,
            posted_at,
            reference,
            note
        ],
    )?;
    audit_tx(
        &tx,
        "journal_entry_created",
        "journal_entry",
        Some(&id),
        json!({
            "entryType": input.entry_type,
            "amount": input.amount,
            "signedAmount": signed_amount,
            "paymentAccount": input.payment_account
        }),
    )?;
    tx.commit()?;
    connection
        .query_row(
            "SELECT j.id, j.entry_type, j.amount, j.signed_amount, j.payment_account,
                    j.occurred_at, j.posted_at, j.reference, j.note, j.reverses_id, 0
             FROM journal_entries j WHERE j.id = ?1",
            [&id],
            map_journal_row,
        )
        .map_err(AppError::from)
}

pub fn reverse_journal_entry(
    connection: &mut Connection,
    input: ReverseEntryInput,
) -> AppResult<JournalEntry> {
    let reason = clean_required(&input.reason, "Le motif", 3)?;
    let original: JournalEntry = connection
        .query_row(
            "SELECT j.id, j.entry_type, j.amount, j.signed_amount, j.payment_account,
                    j.occurred_at, j.posted_at, j.reference, j.note, j.reverses_id,
                    EXISTS(SELECT 1 FROM journal_entries r WHERE r.reverses_id = j.id)
             FROM journal_entries j WHERE j.id = ?1",
            [&input.entry_id],
            map_journal_row,
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    if original.reverses_id.is_some() || original.reversed {
        return Err(AppError::Validation(
            "Cette écriture est déjà une correction ou a déjà été corrigée.".into(),
        ));
    }
    let signed_amount = original
        .signed_amount
        .checked_neg()
        .ok_or_else(|| AppError::Validation("Montant de correction invalide.".into()))?;
    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    let tx = connection.transaction()?;
    tx.execute(
        "INSERT INTO journal_entries
         (id, entry_type, amount, signed_amount, payment_account, occurred_at, posted_at, reference, note, reverses_id)
         VALUES (?1, 'reversal', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            original.amount,
            signed_amount,
            original.payment_account,
            Utc::now().date_naive().format("%Y-%m-%d").to_string(),
            timestamp,
            format!("Correction {}", original.id),
            reason,
            original.id
        ],
    )?;
    audit_tx(
        &tx,
        "journal_entry_reversed",
        "journal_entry",
        Some(&original.id),
        json!({ "reversalId": id, "reason": reason }),
    )?;
    tx.commit()?;
    connection
        .query_row(
            "SELECT j.id, j.entry_type, j.amount, j.signed_amount, j.payment_account,
                    j.occurred_at, j.posted_at, j.reference, j.note, j.reverses_id, 0
             FROM journal_entries j WHERE j.id = ?1",
            [&id],
            map_journal_row,
        )
        .map_err(AppError::from)
}

fn payment_for_debt(connection: &Connection, debt_id: &str) -> AppResult<Vec<DebtPayment>> {
    let mut statement = connection.prepare(
        "SELECT id, debt_id, amount, account, paid_at, note, created_at
         FROM debt_payments WHERE debt_id = ?1 ORDER BY paid_at DESC, created_at DESC",
    )?;
    let values = statement
        .query_map([debt_id], |row| {
            Ok(DebtPayment {
                id: row.get(0)?,
                debt_id: row.get(1)?,
                amount: row.get(2)?,
                account: row.get(3)?,
                paid_at: row.get(4)?,
                note: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(values)
}

fn display_debt_status(
    base: &str,
    remaining: Money,
    due_date: Option<&str>,
    today: NaiveDate,
) -> String {
    if base == "cancelled" {
        return "cancelled".into();
    }
    if remaining == 0 {
        return "paid".into();
    }
    let today = today.format("%Y-%m-%d").to_string();
    if due_date.is_some_and(|due| due < today.as_str()) {
        "overdue".into()
    } else if base == "partial" {
        "partial".into()
    } else {
        "open".into()
    }
}

pub fn list_debts(
    connection: &Connection,
    filters: Option<&ReportFilters>,
) -> AppResult<Vec<Debt>> {
    let mut statement = connection.prepare(
        "SELECT id, customer_name, phone, provider, principal, remaining, issued_at,
                due_date, note, status, created_at
         FROM debts ORDER BY
           CASE status WHEN 'open' THEN 0 WHEN 'partial' THEN 1 WHEN 'paid' THEN 2 ELSE 3 END,
           COALESCE(due_date, '9999-12-31'), issued_at DESC",
    )?;
    let raw = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Money>(4)?,
                row.get::<_, Money>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let today = Utc::now().date_naive();
    let mut debts = Vec::with_capacity(raw.len());
    for (
        id,
        customer_name,
        phone,
        provider,
        principal,
        remaining,
        issued_at,
        due_date,
        note,
        base_status,
        created_at,
    ) in raw
    {
        if filters.is_some_and(|f| !date_in_filter(&issued_at, f)) {
            continue;
        }
        debts.push(Debt {
            payments: payment_for_debt(connection, &id)?,
            status: display_debt_status(&base_status, remaining, due_date.as_deref(), today),
            id,
            customer_name,
            phone,
            provider,
            principal,
            remaining,
            issued_at,
            due_date,
            note,
            created_at,
        });
    }
    Ok(debts)
}

pub fn create_debt(connection: &mut Connection, input: CreateDebtInput) -> AppResult<Debt> {
    let customer_name = clean_required(&input.customer_name, "Le nom du client", 2)?;
    let phone = clean_required(&input.phone, "Le téléphone", 6)?;
    validate_debt_provider(&input.provider)?;
    if input.amount <= 0 {
        return Err(AppError::Validation(
            "Le montant de la dette doit être supérieur à zéro.".into(),
        ));
    }
    let issued_at = validate_date(&input.issued_at, "La date du prêt")?;
    let due_date = match clean_optional(input.due_date) {
        Some(value) => {
            let value = validate_date(&value, "L’échéance")?;
            if value < issued_at {
                return Err(AppError::Validation(
                    "L’échéance ne peut pas précéder la date du prêt.".into(),
                ));
            }
            Some(value)
        }
        None => None,
    };
    let note = clean_optional(input.note);
    let id = Uuid::new_v4().to_string();
    let created_at = now();
    let tx = connection.transaction()?;
    tx.execute(
        "INSERT INTO debts
         (id, customer_name, phone, provider, principal, remaining, issued_at, due_date, note, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7, ?8, 'open', ?9)",
        params![
            id,
            customer_name,
            phone,
            input.provider,
            input.amount,
            issued_at,
            due_date,
            note,
            created_at
        ],
    )?;
    audit_tx(
        &tx,
        "debt_created",
        "debt",
        Some(&id),
        json!({
            "customerName": customer_name,
            "phone": phone,
            "provider": input.provider,
            "amount": input.amount,
            "dueDate": due_date
        }),
    )?;
    tx.commit()?;
    list_debts(connection, None)?
        .into_iter()
        .find(|debt| debt.id == id)
        .ok_or(AppError::NotFound)
}

pub fn record_debt_payment(
    connection: &mut Connection,
    input: RecordPaymentInput,
) -> AppResult<Debt> {
    if input.amount <= 0 {
        return Err(AppError::Validation(
            "Le remboursement doit être supérieur à zéro.".into(),
        ));
    }
    validate_payment_account(&input.account)?;
    let paid_at = validate_date(&input.paid_at, "La date du remboursement")?;
    let note = clean_optional(input.note);
    let current: (Money, String) = connection
        .query_row(
            "SELECT remaining, status FROM debts WHERE id = ?1",
            [&input.debt_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    if current.1 == "cancelled" || current.0 == 0 {
        return Err(AppError::Validation(
            "Cette dette est déjà clôturée.".into(),
        ));
    }
    if input.amount > current.0 {
        return Err(AppError::Validation(format!(
            "Le remboursement dépasse le solde restant de {} FCFA.",
            current.0
        )));
    }
    let remaining = current.0 - input.amount;
    let status = if remaining == 0 { "paid" } else { "partial" };
    let payment_id = Uuid::new_v4().to_string();
    let created_at = now();
    let tx = connection.transaction()?;
    tx.execute(
        "INSERT INTO debt_payments (id, debt_id, amount, account, paid_at, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            payment_id,
            input.debt_id,
            input.amount,
            input.account,
            paid_at,
            note,
            created_at
        ],
    )?;
    tx.execute(
        "UPDATE debts SET remaining = ?1, status = ?2 WHERE id = ?3",
        params![remaining, status, input.debt_id],
    )?;
    audit_tx(
        &tx,
        "debt_payment_recorded",
        "debt",
        Some(&input.debt_id),
        json!({
            "paymentId": payment_id,
            "amount": input.amount,
            "remaining": remaining,
            "account": input.account
        }),
    )?;
    tx.commit()?;
    list_debts(connection, None)?
        .into_iter()
        .find(|debt| debt.id == input.debt_id)
        .ok_or(AppError::NotFound)
}

pub fn cancel_debt(connection: &mut Connection, input: CancelDebtInput) -> AppResult<Debt> {
    let reason = clean_required(&input.reason, "Le motif d’annulation", 3)?;
    let status: String = connection
        .query_row(
            "SELECT status FROM debts WHERE id = ?1",
            [&input.debt_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    if status == "cancelled" {
        return Err(AppError::Validation("Cette dette est déjà annulée.".into()));
    }
    let tx = connection.transaction()?;
    tx.execute(
        "UPDATE debts SET remaining = 0, status = 'cancelled', cancellation_reason = ?1 WHERE id = ?2",
        params![reason, input.debt_id],
    )?;
    audit_tx(
        &tx,
        "debt_cancelled",
        "debt",
        Some(&input.debt_id),
        json!({ "reason": reason }),
    )?;
    tx.commit()?;
    list_debts(connection, None)?
        .into_iter()
        .find(|debt| debt.id == input.debt_id)
        .ok_or(AppError::NotFound)
}

pub fn get_dashboard(connection: &Connection) -> AppResult<Dashboard> {
    let settings = get_settings(connection)?;
    let last_inventory = last_inventory(connection)?;
    let journal_net = journal_net_after(connection, &last_inventory.closed_at)?;
    let expected_capital = last_inventory
        .actual_total
        .checked_add(journal_net)
        .ok_or_else(|| {
            AppError::Validation("Le capital attendu dépasse la limite autorisée.".into())
        })?;
    let open_receivables = open_receivables(connection)?;
    let debts = list_debts(connection, None)?;
    let open_debts_count = debts
        .iter()
        .filter(|debt| matches!(debt.status.as_str(), "open" | "partial" | "overdue"))
        .count() as i64;
    let overdue_debts_count = debts.iter().filter(|debt| debt.status == "overdue").count() as i64;
    let closed_at = DateTime::parse_from_rfc3339(&last_inventory.closed_at)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .with_timezone(&Utc);
    let next = closed_at + Duration::minutes(settings.inventory_interval_minutes);
    Ok(Dashboard {
        settings,
        expected_capital,
        last_actual_capital: last_inventory.actual_total,
        open_receivables,
        open_debts_count,
        overdue_debts_count,
        journal_net_since_inventory: journal_net,
        next_inventory_at: next.to_rfc3339(),
        inventory_overdue: Utc::now() > next,
        last_inventory,
    })
}

pub fn list_audit_events(connection: &Connection, limit: i64) -> AppResult<Vec<AuditEvent>> {
    let limit = limit.clamp(1, 500);
    let mut statement = connection.prepare(
        "SELECT id, action, entity_type, entity_id, details_json, occurred_at
         FROM audit_events ORDER BY occurred_at DESC LIMIT ?1",
    )?;
    let events = statement
        .query_map([limit], |row| {
            let details: String = row.get(4)?;
            Ok(AuditEvent {
                id: row.get(0)?,
                action: row.get(1)?,
                entity_type: row.get(2)?,
                entity_id: row.get(3)?,
                details: serde_json::from_str(&details).unwrap_or_else(|_| json!({})),
                occurred_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(events)
}

pub fn get_report(connection: &Connection, filters: ReportFilters) -> AppResult<ReportData> {
    if let (Some(from), Some(to)) = (&filters.from, &filters.to) {
        validate_date(from, "La date de début")?;
        validate_date(to, "La date de fin")?;
        if from > to {
            return Err(AppError::Validation(
                "La date de début doit précéder la date de fin.".into(),
            ));
        }
    }
    let inventories = list_inventories(connection, Some(&filters))?;
    let journal = list_journal_entries(connection, Some(&filters))?;
    let debts = list_debts(connection, Some(&filters))?;
    let total_positive = journal
        .iter()
        .filter(|entry| entry.signed_amount > 0)
        .map(|entry| entry.signed_amount)
        .sum();
    let total_negative = journal
        .iter()
        .filter(|entry| entry.signed_amount < 0)
        .map(|entry| entry.signed_amount)
        .sum();
    let total_variance = inventories.iter().map(|item| item.variance).sum();
    let outstanding_receivables = open_receivables(connection)?;
    Ok(ReportData {
        generated_at: now(),
        filters,
        inventories,
        journal,
        debts,
        total_positive,
        total_negative,
        total_variance,
        outstanding_receivables,
    })
}

pub fn integrity_check(connection: &Connection) -> AppResult<()> {
    let result: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(AppError::Database(rusqlite::Error::InvalidQuery))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        // Les tests ci-dessous vérifient les règles comptables, pas le
        // chiffrement. Une base en mémoire évite de lancer plusieurs KDF
        // SQLCipher/OpenSSL en parallèle sur les petits threads de libtest.
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();
        connection
    }

    fn setup(connection: &mut Connection) {
        let input = SetupInput {
            business_name: "Boutique test".into(),
            pin: "123456".into(),
            recovery_password: "mot de passe de récupération".into(),
            initial_capital: 5_000_000,
            orange_money: 1_500_000,
            wave: 1_200_000,
            djamo: 800_000,
            cash: 1_500_000,
        };
        let balances = AccountBalances {
            orange_money: input.orange_money,
            wave: input.wave,
            djamo: input.djamo,
            cash: input.cash,
        };
        initialize_business(connection, &input, &balances).unwrap();
    }

    #[test]
    fn journal_updates_expected_capital() {
        let mut connection = test_db();
        setup(&mut connection);
        create_journal_entry(
            &mut connection,
            CreateJournalEntryInput {
                entry_type: "sale".into(),
                amount: 100_000,
                payment_account: "cash".into(),
                occurred_at: "2026-08-26".into(),
                reference: None,
                note: None,
            },
        )
        .unwrap();
        create_journal_entry(
            &mut connection,
            CreateJournalEntryInput {
                entry_type: "expense".into(),
                amount: 30_000,
                payment_account: "cash".into(),
                occurred_at: "2026-08-26".into(),
                reference: None,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(
            get_dashboard(&connection).unwrap().expected_capital,
            5_070_000
        );
    }

    #[test]
    fn debt_and_partial_payment_preserve_receivable_logic() {
        let mut connection = test_db();
        setup(&mut connection);
        let debt = create_debt(
            &mut connection,
            CreateDebtInput {
                customer_name: "Awa Ndiaye".into(),
                phone: "771234567".into(),
                provider: "wave".into(),
                amount: 50_000,
                issued_at: "2026-08-26".into(),
                // Les échéances sont testées séparément avec une date contrôlée.
                due_date: None,
                note: None,
            },
        )
        .unwrap();
        assert_eq!(open_receivables(&connection).unwrap(), 50_000);

        let updated = record_debt_payment(
            &mut connection,
            RecordPaymentInput {
                debt_id: debt.id,
                amount: 20_000,
                account: "cash".into(),
                paid_at: "2026-08-27".into(),
                note: None,
            },
        )
        .unwrap();
        assert_eq!(updated.remaining, 30_000);
        assert_eq!(updated.status, "partial");
        assert_eq!(open_receivables(&connection).unwrap(), 30_000);
    }

    #[test]
    fn debt_status_changes_only_after_its_due_date() {
        let due_date = Some("2026-08-30");
        let cases = [
            ("2026-08-29", false),
            ("2026-08-30", false),
            ("2026-08-31", true),
            ("2026-09-05", true),
            ("2099-01-01", true),
        ];

        for (date, overdue) in cases {
            let today = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
            for base_status in ["open", "partial"] {
                let expected = if overdue { "overdue" } else { base_status };
                assert_eq!(
                    display_debt_status(base_status, 30_000, due_date, today),
                    expected,
                    "statut {base_status} au {date}"
                );
                assert_eq!(
                    display_debt_status(base_status, 30_000, None, today),
                    base_status,
                    "une dette sans échéance ne devient pas en retard au {date}"
                );
            }
        }
    }

    #[test]
    fn settled_and_cancelled_debts_are_not_overdue() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 5).unwrap();
        let due_date = Some("2026-08-30");
        assert_eq!(display_debt_status("paid", 0, due_date, today), "paid");
        assert_eq!(
            display_debt_status("cancelled", 0, due_date, today),
            "cancelled"
        );
    }

    #[test]
    fn overpayment_is_rejected() {
        let mut connection = test_db();
        setup(&mut connection);
        let debt = create_debt(
            &mut connection,
            CreateDebtInput {
                customer_name: "Moussa Fall".into(),
                phone: "781234567".into(),
                provider: "orange_money".into(),
                amount: 10_000,
                issued_at: "2026-08-26".into(),
                due_date: None,
                note: None,
            },
        )
        .unwrap();
        assert!(record_debt_payment(
            &mut connection,
            RecordPaymentInput {
                debt_id: debt.id,
                amount: 10_001,
                account: "cash".into(),
                paid_at: "2026-08-27".into(),
                note: None,
            }
        )
        .is_err());
    }

    #[test]
    fn non_zero_inventory_variance_requires_explanation_and_becomes_baseline() {
        let mut connection = test_db();
        setup(&mut connection);
        let invalid = close_inventory(
            &mut connection,
            CloseInventoryInput {
                orange_money: 1_510_000,
                wave: 1_200_000,
                djamo: 800_000,
                cash: 1_500_000,
                variance_category: None,
                variance_note: None,
            },
        );
        assert!(invalid.is_err());
        let closed = close_inventory(
            &mut connection,
            CloseInventoryInput {
                orange_money: 1_510_000,
                wave: 1_200_000,
                djamo: 800_000,
                cash: 1_500_000,
                variance_category: Some("commission_mobile".into()),
                variance_note: Some("Commissions de la période".into()),
            },
        )
        .unwrap();
        assert_eq!(closed.variance, 10_000);
        assert_eq!(
            get_dashboard(&connection).unwrap().expected_capital,
            5_010_000
        );
    }

    #[test]
    fn inventory_correction_is_linked_and_updates_expected_capital() {
        let mut connection = test_db();
        setup(&mut connection);
        let inventory = last_inventory(&connection).unwrap();
        let entry = create_inventory_correction(
            &mut connection,
            InventoryCorrectionInput {
                inventory_id: inventory.id.clone(),
                amount: 25_000,
                direction: "decrease".into(),
                payment_account: "cash".into(),
                reason: "Correction d’un comptage erroné".into(),
            },
        )
        .unwrap();
        assert_eq!(entry.signed_amount, -25_000);
        let expected_reference = format!("inventory:{}", inventory.id);
        assert_eq!(
            entry.reference.as_deref(),
            Some(expected_reference.as_str())
        );
        assert_eq!(
            get_dashboard(&connection).unwrap().expected_capital,
            4_975_000
        );
    }

    #[test]
    fn encrypted_database_roundtrip_uses_sqlcipher() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("encrypted.db");
        let key = [9u8; 32];

        {
            let connection = open_database(&path, &key).unwrap();
            migrate(&connection).unwrap();
            integrity_check(&connection).unwrap();
            let memory_security: String = connection
                .query_row("PRAGMA cipher_memory_security", [], |row| row.get(0))
                .unwrap();
            assert!(matches!(
                memory_security.to_ascii_lowercase().as_str(),
                "0" | "off"
            ));
        }

        let reopened = open_database(&path, &key).unwrap();
        integrity_check(&reopened).unwrap();
        drop(reopened);

        assert!(matches!(
            open_database(&path, &[8u8; 32]),
            Err(AppError::InvalidPin)
        ));
    }
}
