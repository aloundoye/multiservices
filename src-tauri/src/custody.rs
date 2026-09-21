use std::collections::{BTreeMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    accounts, db,
    domain::{clean_optional, clean_required},
    error::{AppError, AppResult},
    models::*,
};

const MAX: i64 = 9_007_199_254_740_991;
fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}
pub(crate) fn bounded(value: i64) -> AppResult<i64> {
    if !(-MAX..=MAX).contains(&value) {
        return Err(invalid("Limite de calcul FCFA dépassée."));
    }
    Ok(value)
}
pub(crate) fn add(a: i64, b: i64) -> AppResult<i64> {
    bounded(
        a.checked_add(b)
            .ok_or_else(|| invalid("Limite de calcul FCFA dépassée."))?,
    )
}
fn positive(value: i64) -> AppResult<i64> {
    bounded(value)?;
    if value <= 0 {
        return Err(invalid(
            "Saisissez un montant FCFA entier supérieur à zéro.",
        ));
    }
    Ok(value)
}

// Results and request fingerprints commit with the business operation and audit.
fn once<I: Serialize, T: Serialize + DeserializeOwned>(
    connection: &mut Connection,
    request_id: &str,
    action: &str,
    input: &I,
    work: impl FnOnce(&Transaction<'_>) -> AppResult<T>,
) -> AppResult<T> {
    Uuid::parse_str(request_id).map_err(|_| invalid("Identifiant d’opération invalide."))?;
    let payload = serde_json::to_string(input)?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let prior: Option<(String, String, String)> = tx
        .query_row(
            "SELECT action,input_json,result_json FROM custody_requests WHERE id=?1",
            [request_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((previous_action, previous_payload, result)) = prior {
        if previous_action != action || previous_payload != payload {
            return Err(invalid(
                "Cet identifiant a déjà été utilisé avec d’autres données. Rouvrez le formulaire.",
            ));
        }
        return Ok(serde_json::from_str(&result)?);
    }
    let result = work(&tx)?;
    total(&tx)?;
    bounded(db::get_dashboard(&tx)?.expected_capital)?;
    tx.execute(
        "INSERT INTO custody_requests VALUES (?1,?2,?3,?4)",
        params![request_id, action, payload, serde_json::to_string(&result)?],
    )?;
    tx.commit()?;
    Ok(result)
}

pub fn customers(c: &Connection) -> AppResult<Vec<CustodyCustomer>> {
    let mut stmt = c.prepare("SELECT id,name,phone,active,balance FROM custody_customers ORDER BY active DESC,name COLLATE NOCASE,id")?;
    let mut result = stmt
        .query_map([], |r| {
            Ok(CustodyCustomer {
                id: r.get(0)?,
                name: r.get(1)?,
                phone: r.get(2)?,
                active: r.get(3)?,
                balance: r.get(4)?,
                total_received: 0,
                total_withdrawn: 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let all = movements(c)?;
    for customer in &mut result {
        for m in all
            .iter()
            .filter(|m| m.customer_id == customer.id && !m.reversed && m.kind != "reversal")
        {
            if m.delta > 0 {
                customer.total_received = add(customer.total_received, m.delta)?;
            } else {
                customer.total_withdrawn = add(customer.total_withdrawn, -m.delta)?;
            }
        }
    }
    Ok(result)
}
fn customer(c: &Connection, id: &str, active: bool) -> AppResult<CustodyCustomer> {
    let found = customers(c)?
        .into_iter()
        .find(|v| v.id == id)
        .ok_or(AppError::NotFound)?;
    if active && !found.active {
        return Err(invalid("Ce client est archivé."));
    }
    Ok(found)
}
pub fn save_customer(
    c: &mut Connection,
    input: SaveCustodyCustomerInput,
) -> AppResult<CustodyCustomer> {
    once(c, &input.request_id, "customer_saved", &input, |tx| {
        let name = clean_required(&input.name, "Le nom du client", 1)?;
        let phone = clean_optional(input.phone.clone());
        let id = input
            .customer_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        if input.customer_id.is_some() {
            let current = customer(tx, &id, false)?;
            if !input.active && current.balance != 0 {
                return Err(invalid(
                    "Restituez tout le solde avant d’archiver ce client.",
                ));
            }
            tx.execute(
                "UPDATE custody_customers SET name=?2,phone=?3,active=?4 WHERE id=?1",
                params![id, name, phone, input.active],
            )?;
        } else {
            tx.execute("INSERT INTO custody_customers(id,name,phone,active,created_at) VALUES (?1,?2,?3,?4,?5)",params![id,name,phone,input.active,db::now()])?;
        }
        db::audit_tx(
            tx,
            "custody_customer_saved",
            "custody_customer",
            Some(&id),
            json!({"name":name,"phone":phone,"active":input.active}),
        )?;
        customer(tx, &id, false)
    })
}

pub fn movements(c: &Connection) -> AppResult<Vec<CustodyMovement>> {
    let mut stmt = c.prepare("SELECT m.sequence,m.id,m.customer_id,m.customer_name,m.customer_phone,m.kind,m.delta,m.capital_adjustment,m.balance_after,m.account_json,m.occurred_at,m.posted_at,m.operator,m.note,m.reverses_id,EXISTS(SELECT 1 FROM custody_movements r WHERE r.reverses_id=m.id) FROM custody_movements m ORDER BY m.sequence DESC")?;
    let rows = stmt
        .query_map([], |r| {
            let account: Option<String> = r.get(9)?;
            Ok((
                CustodyMovement {
                    sequence: r.get(0)?,
                    id: r.get(1)?,
                    customer_id: r.get(2)?,
                    customer_name: r.get(3)?,
                    customer_phone: r.get(4)?,
                    kind: r.get(5)?,
                    delta: r.get(6)?,
                    capital_adjustment: r.get(7)?,
                    balance_after: r.get(8)?,
                    account_snapshot: None,
                    occurred_at: r.get(10)?,
                    posted_at: r.get(11)?,
                    operator: r.get(12)?,
                    note: r.get(13)?,
                    reverses_id: r.get(14)?,
                    reversed: r.get(15)?,
                },
                account,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(mut m, account)| {
            m.account_snapshot = account.map(|v| serde_json::from_str(&v)).transpose()?;
            Ok(m)
        })
        .collect()
}
fn insert_movement(tx: &Transaction<'_>, mut m: CustodyMovement) -> AppResult<CustodyMovement> {
    let client = customer(tx, &m.customer_id, false)?;
    m.balance_after = add(client.balance, m.delta)?;
    if m.balance_after < 0 {
        return Err(invalid("Le montant dépasse le solde gardé pour ce client."));
    }
    tx.execute("UPDATE custody_customers SET balance=?2,active=CASE WHEN ?2>0 THEN 1 ELSE active END WHERE id=?1",params![m.customer_id,m.balance_after])?;
    let account_json = m
        .account_snapshot
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    tx.execute("INSERT INTO custody_movements(id,customer_id,customer_name,customer_phone,kind,delta,capital_adjustment,balance_after,account_id,account_json,occurred_at,posted_at,operator,note,reverses_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![m.id,m.customer_id,m.customer_name,m.customer_phone,m.kind,m.delta,m.capital_adjustment,m.balance_after,m.account_snapshot.as_ref().map(|a| &a.account_id),account_json,m.occurred_at,m.posted_at,m.operator,m.note,m.reverses_id])?;
    m.sequence = tx.last_insert_rowid();
    db::audit_tx(
        tx,
        "custody_movement_recorded",
        "custody_movement",
        Some(&m.id),
        serde_json::to_value(&m)?,
    )?;
    // Include cumulative totals in numeric validation, not just the current balance.
    customer(tx, &m.customer_id, false)?;
    Ok(m)
}
fn draft(client: &CustodyCustomer, kind: &str, delta: i64) -> CustodyMovement {
    CustodyMovement {
        sequence: 0,
        id: Uuid::new_v4().to_string(),
        customer_id: client.id.clone(),
        customer_name: client.name.clone(),
        customer_phone: client.phone.clone(),
        kind: kind.into(),
        delta,
        capital_adjustment: 0,
        balance_after: 0,
        account_snapshot: None,
        occurred_at: db::now(),
        posted_at: db::now(),
        operator: "Gérant".into(),
        note: None,
        reverses_id: None,
        reversed: false,
    }
}
pub fn record(c: &mut Connection, input: CreateCustodyMovementInput) -> AppResult<CustodyMovement> {
    once(c, &input.request_id, "custody_recorded", &input, |tx| {
        let amount = positive(input.amount)?;
        let delta = match input.kind.as_str() {
            "deposit" => amount,
            "withdrawal" => -amount,
            _ => return Err(invalid("Type de mouvement invalide.")),
        };
        let client = customer(tx, &input.customer_id, true)?;
        let mut m = draft(&client, &input.kind, delta);
        m.account_snapshot = Some(accounts::get(tx, &input.account_id, true)?.snapshot);
        m.occurred_at = db::validate_date(&input.occurred_at, "La date du mouvement")?;
        m.note = clean_optional(input.note.clone());
        insert_movement(tx, m)
    })
}
pub fn reverse(
    c: &mut Connection,
    input: ReverseCustodyMovementInput,
) -> AppResult<CustodyMovement> {
    once(c, &input.request_id, "custody_reversed", &input, |tx| {
        let reason = clean_required(&input.reason, "Le motif d’annulation", 3)?;
        let original = movements(tx)?
            .into_iter()
            .find(|m| m.id == input.movement_id)
            .ok_or(AppError::NotFound)?;
        if original.reversed || original.kind == "reversal" {
            return Err(invalid(
                "Ce mouvement est déjà annulé ou est une annulation.",
            ));
        }
        let client = customer(tx, &original.customer_id, false)?;
        let mut m = draft(&client, "reversal", -original.delta);
        m.capital_adjustment = -original.capital_adjustment;
        m.account_snapshot = original.account_snapshot;
        m.note = Some(reason);
        m.reverses_id = Some(original.id);
        insert_movement(tx, m)
    })
}

pub fn preview_opening(
    c: &Connection,
    input: &CustodyOpeningInput,
) -> AppResult<CustodyOpeningPreview> {
    if input.lines.is_empty() {
        return Err(invalid("Ajoutez au moins un client à la reprise."));
    }
    let mut seen = HashSet::new();
    let mut total = 0;
    let all = movements(c)?;
    for line in &input.lines {
        customer(c, &line.customer_id, true)?;
        if !seen.insert(&line.customer_id) {
            return Err(invalid(
                "Chaque client ne peut figurer qu’une fois dans la reprise.",
            ));
        }
        if all
            .iter()
            .any(|m| m.customer_id == line.customer_id && m.kind == "opening" && !m.reversed)
        {
            return Err(invalid(
                "Une reprise existe déjà pour ce client. Annulez-la avant de la remplacer.",
            ));
        }
        total = add(total, positive(line.amount)?)?;
    }
    add(self::total(c)?, total)?;
    let expected_capital = db::get_dashboard(c)?.expected_capital;
    Ok(CustodyOpeningPreview {
        total,
        expected_capital,
        corrected_capital: add(expected_capital, -total)?,
    })
}
pub fn opening(c: &mut Connection, input: CustodyOpeningInput) -> AppResult<Vec<CustodyMovement>> {
    once(c, &input.request_id, "custody_opening", &input, |tx| {
        preview_opening(tx, &input)?;
        input
            .lines
            .iter()
            .map(|line| {
                let client = customer(tx, &line.customer_id, true)?;
                let mut m = draft(&client, "opening", line.amount);
                m.capital_adjustment = -line.amount;
                m.note =
                    Some("Argent déjà gardé avant la mise à jour — sans mouvement d’argent".into());
                insert_movement(tx, m)
            })
            .collect()
    })
}
pub fn total(c: &Connection) -> AppResult<i64> {
    let value = c.query_row(
        "SELECT COALESCE(SUM(balance),0) FROM custody_customers",
        [],
        |r| r.get(0),
    )?;
    bounded(value)
}
pub fn count(c: &Connection) -> AppResult<i64> {
    Ok(c.query_row(
        "SELECT count(*) FROM custody_customers WHERE balance>0",
        [],
        |r| r.get(0),
    )?)
}
pub fn capital_after(c: &Connection, sequence: i64) -> AppResult<i64> {
    let mut stmt = c.prepare(
        "SELECT capital_adjustment FROM custody_movements WHERE sequence>?1 ORDER BY sequence",
    )?;
    let amounts = stmt
        .query_map([sequence], |r| r.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    amounts.into_iter().try_fold(0, add)
}
pub fn sequence(c: &Connection) -> AppResult<i64> {
    Ok(c.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM custody_movements",
        [],
        |r| r.get(0),
    )?)
}
pub fn freeze(tx: &Transaction<'_>, inventory_id: &str) -> AppResult<()> {
    tx.execute("INSERT INTO inventory_custody_balances SELECT ?1,id,name,phone,balance FROM custody_customers WHERE balance>0",[inventory_id])?;
    Ok(())
}
pub fn inventory_balances(c: &Connection, id: &str) -> AppResult<Vec<CustodyBalanceSnapshot>> {
    let mut stmt=c.prepare("SELECT customer_id,customer_name,customer_phone,balance FROM inventory_custody_balances WHERE inventory_id=?1 ORDER BY customer_name,customer_id")?;
    let result = stmt
        .query_map([id], |r| {
            Ok(CustodyBalanceSnapshot {
                customer_id: r.get(0)?,
                customer_name: r.get(1)?,
                customer_phone: r.get(2)?,
                balance: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result)
}

// Business periods use the declared date; inventory cutoffs always use the immutable sequence.
pub fn report(c: &Connection, filters: &ReportFilters) -> AppResult<CustodyReport> {
    let all = movements(c)?;
    let mut balances = BTreeMap::<String, CustodyPeriodBalance>::new();
    let mut selected = Vec::new();
    for m in all.iter().rev() {
        let date = m.occurred_at.get(..10).unwrap_or(&m.occurred_at);
        if filters.to.as_deref().is_some_and(|to| date > to) {
            continue;
        }
        let row = balances
            .entry(m.customer_id.clone())
            .or_insert_with(|| CustodyPeriodBalance {
                customer_id: m.customer_id.clone(),
                customer_name: m.customer_name.clone(),
                opening_balance: 0,
                increases: 0,
                decreases: 0,
                closing_balance: 0,
            });
        row.customer_name = m.customer_name.clone();
        if filters.from.as_deref().is_some_and(|from| date < from) {
            row.opening_balance = add(row.opening_balance, m.delta)?;
        } else {
            if m.delta > 0 {
                row.increases = add(row.increases, m.delta)?;
            } else {
                row.decreases = add(row.decreases, -m.delta)?;
            }
            selected.push(m.clone());
        }
        row.closing_balance = add(row.closing_balance, m.delta)?;
    }
    let balances: Vec<_> = balances.into_values().collect();
    let opening_balance = balances
        .iter()
        .try_fold(0, |sum, b| add(sum, b.opening_balance))?;
    let closing_balance = balances
        .iter()
        .try_fold(0, |sum, b| add(sum, b.closing_balance))?;
    selected.reverse();
    Ok(CustodyReport {
        balances,
        movements: selected,
        opening_balance,
        closing_balance,
    })
}

#[cfg(test)]
#[path = "custody_tests.rs"]
mod tests;
