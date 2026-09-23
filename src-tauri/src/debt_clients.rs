use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    accounts,
    custody::{add, bounded},
    db,
    domain::{clean_optional, clean_required, validate_debt_provider},
    error::{AppError, AppResult},
    models::*,
};

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}
fn positive(value: Money) -> AppResult<()> {
    bounded(value)?;
    if value <= 0 {
        return Err(invalid(
            "Saisissez un montant FCFA entier supérieur à zéro.",
        ));
    }
    Ok(())
}
fn identity(name: &str, phone: &str) -> (String, String) {
    (
        name.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase(),
        phone
            .chars()
            .filter(|c| !c.is_whitespace() && !['-', '(', ')'].contains(c))
            .collect(),
    )
}

// Only exact normalized pairs are grouped. All historical debt/payment fields remain untouched.
pub(crate) fn migrate_customers(c: &Connection) -> AppResult<()> {
    let mut ids = HashMap::new();
    let mut stmt = c.prepare("SELECT id,name,phone FROM debt_customers ORDER BY created_at,id")?;
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, name, phone) = row?;
        ids.entry(identity(&name, &phone)).or_insert(id);
    }
    let mut stmt = c.prepare("SELECT id,customer_name,phone,created_at FROM debts WHERE customer_id IS NULL ORDER BY created_at,id")?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (debt_id, name, phone, created_at) in rows {
        let key = identity(&name, &phone);
        let id = if let Some(id) = ids.get(&key) {
            id.clone()
        } else {
            let id = Uuid::new_v4().to_string();
            c.execute(
                "INSERT INTO debt_customers VALUES (?1,?2,?3,1,?4)",
                params![id, name.trim(), phone.trim(), created_at],
            )?;
            ids.insert(key, id.clone());
            id
        };
        c.execute(
            "UPDATE debts SET customer_id=?2 WHERE id=?1",
            params![debt_id, id],
        )?;
    }
    Ok(())
}

fn once<I: Serialize, T: Serialize + DeserializeOwned>(
    c: &mut Connection,
    request: &str,
    action: &str,
    input: &I,
    work: impl FnOnce(&Transaction<'_>) -> AppResult<T>,
) -> AppResult<T> {
    Uuid::parse_str(request).map_err(|_| invalid("Identifiant d’opération invalide."))?;
    let payload = serde_json::to_string(input)?;
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let previous: Option<(String, String, String)> = tx
        .query_row(
            "SELECT action,input_json,result_json FROM debt_requests WHERE id=?1",
            [request],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((a, p, result)) = previous {
        if a != action || p != payload {
            return Err(invalid(
                "Cet identifiant a déjà été utilisé avec d’autres données. Rouvrez le formulaire.",
            ));
        }
        return Ok(serde_json::from_str(&result)?);
    }
    let result = work(&tx)?;
    // Bound the aggregate too, so inventory/JS calculations stay exact.
    customers(&tx)?
        .iter()
        .try_fold(0, |sum, c| add(sum, c.remaining))?;
    bounded(db::get_dashboard(&tx)?.open_receivables)?;
    tx.execute(
        "INSERT INTO debt_requests VALUES (?1,?2,?3,?4)",
        params![request, action, payload, serde_json::to_string(&result)?],
    )?;
    tx.commit()?;
    Ok(result)
}

pub fn customers(c: &Connection) -> AppResult<Vec<DebtCustomer>> {
    let mut stmt = c.prepare("SELECT id,name,phone,active FROM debt_customers ORDER BY active DESC,name COLLATE NOCASE,id")?;
    let mut result = stmt
        .query_map([], |r| {
            Ok(DebtCustomer {
                id: r.get(0)?,
                name: r.get(1)?,
                phone: r.get(2)?,
                active: r.get(3)?,
                remaining: 0,
                total_repaid: 0,
                overdue_count: 0,
                overdue_amount: 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let debts = db::list_debts(c, None)?;
    for customer in &mut result {
        for debt in debts.iter().filter(|d| d.customer_id == customer.id) {
            if debt.status != "cancelled" {
                customer.remaining = add(customer.remaining, debt.remaining)?;
            }
            if debt.status == "overdue" {
                customer.overdue_count += 1;
                customer.overdue_amount = add(customer.overdue_amount, debt.remaining)?;
            }
            for payment in &debt.payments {
                customer.total_repaid = add(customer.total_repaid, payment.amount)?;
            }
        }
    }
    Ok(result)
}
fn customer(c: &Connection, id: &str, active: bool) -> AppResult<DebtCustomer> {
    let found = customers(c)?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or(AppError::NotFound)?;
    if active && !found.active {
        return Err(invalid("Ce client est archivé. Réactivez sa fiche."));
    }
    Ok(found)
}
pub fn save_customer(c: &mut Connection, input: SaveDebtCustomerInput) -> AppResult<DebtCustomer> {
    once(c, &input.request_id, "customer_saved", &input, |tx| {
        let name = clean_required(&input.name, "Le nom du client", 2)?;
        let phone = clean_required(&input.phone, "Le téléphone", 6)?;
        let id = input
            .customer_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        if input.customer_id.is_some() {
            let old = customer(tx, &id, false)?;
            if !input.active && old.remaining != 0 {
                return Err(invalid(
                    "Le client doit avoir un solde nul avant l’archivage.",
                ));
            }
            tx.execute(
                "UPDATE debt_customers SET name=?2,phone=?3,active=?4 WHERE id=?1",
                params![id, name, phone, input.active],
            )?;
        } else {
            tx.execute(
                "INSERT INTO debt_customers VALUES (?1,?2,?3,?4,?5)",
                params![id, name, phone, input.active, db::now()],
            )?;
        }
        db::audit_tx(
            tx,
            "debt_customer_saved",
            "debt_customer",
            Some(&id),
            json!({"name":name,"phone":phone,"active":input.active}),
        )?;
        customer(tx, &id, false)
    })
}
pub fn create_debt(c: &mut Connection, input: CreateClientDebtInput) -> AppResult<Debt> {
    once(c, &input.request_id, "debt_created", &input, |tx| {
        positive(input.amount)?;
        let client = customer(tx, &input.customer_id, true)?;
        let account = accounts::get(tx, &input.account_id, true)?.snapshot;
        validate_debt_provider(&account.provider)?;
        let issued = db::validate_date(&input.issued_at, "La date du prêt")?;
        let due = clean_optional(input.due_date.clone())
            .map(|d| db::validate_date(&d, "L’échéance"))
            .transpose()?;
        if due.as_ref().is_some_and(|d| d < &issued) {
            return Err(invalid("L’échéance ne peut pas précéder la date du prêt."));
        }
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO debts(id,customer_id,customer_name,phone,provider,principal,remaining,issued_at,due_date,note,status,created_at,account_id,account_name,account_identifier) VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,?9,'open',?10,?11,?12,?13)",params![id,client.id,client.name,client.phone,account.provider,input.amount,issued,due,clean_optional(input.note.clone()),db::now(),account.account_id,account.name,account.identifier])?;
        db::audit_tx(
            tx,
            "debt_created",
            "debt",
            Some(&id),
            json!({"customerId":client.id,"customerName":client.name,"phone":client.phone,"amount":input.amount,"account":account}),
        )?;
        db::list_debts(tx, None)?
            .into_iter()
            .find(|d| d.id == id)
            .ok_or(AppError::NotFound)
    })
}

pub fn preview(c: &Connection, input: &RepaymentPreviewInput) -> AppResult<RepaymentPreview> {
    positive(input.amount)?;
    customer(c, &input.customer_id, true)?;
    let date = db::validate_date(&input.paid_at, "La date du remboursement")?;
    let mut stmt = c.prepare("SELECT id,issued_at,remaining,created_at FROM debts WHERE customer_id=?1 AND status IN ('open','partial') AND remaining>0 AND issued_at<=?2 ORDER BY issued_at,created_at,id")?;
    let eligible = stmt
        .query_map(params![input.customer_id, date], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Money>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let total = eligible.iter().try_fold(0, |sum, d| add(sum, d.2))?;
    if input.amount > total {
        return Err(invalid(&format!(
            "Le montant dépasse le total remboursable de {total} FCFA à cette date."
        )));
    }
    let token = hex::encode(Sha256::digest(serde_json::to_vec(&(input, &eligible))?));
    let mut left = input.amount;
    let mut allocations = Vec::new();
    for (id, issued, remaining, _) in eligible {
        if left == 0 {
            break;
        }
        let amount = left.min(remaining);
        allocations.push(RepaymentAllocation {
            debt_id: id,
            issued_at: issued,
            amount,
            remaining_after: Some(remaining - amount),
        });
        left -= amount;
    }
    Ok(RepaymentPreview {
        token,
        eligible_total: total,
        allocations,
    })
}

pub fn record_repayment(
    c: &mut Connection,
    input: RecordCustomerRepaymentInput,
) -> AppResult<CustomerRepayment> {
    once(c, &input.request_id, "customer_repayment", &input, |tx| {
        let preview = preview(
            tx,
            &RepaymentPreviewInput {
                customer_id: input.customer_id.clone(),
                amount: input.amount,
                paid_at: input.paid_at.clone(),
            },
        )?;
        if preview.token != input.preview_token {
            return Err(invalid(
                "La répartition a changé. Actualisez l’aperçu avant de valider.",
            ));
        }
        let client = customer(tx, &input.customer_id, true)?;
        let account = accounts::get(tx, &input.account_id, true)?.snapshot;
        let id = Uuid::new_v4().to_string();
        let created = db::now();
        let note = clean_optional(input.note.clone());
        tx.execute(
            "INSERT INTO customer_repayments VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                id,
                client.id,
                client.name,
                client.phone,
                input.amount,
                account.account_id,
                serde_json::to_string(&account)?,
                input.paid_at,
                note,
                created
            ],
        )?;
        for (position, a) in preview.allocations.iter().enumerate() {
            let payment = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO debt_payments(id,debt_id,amount,account,paid_at,note,created_at,account_id,account_name,account_identifier) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![payment,a.debt_id,a.amount,account.provider,input.paid_at,note,created,account.account_id,account.name,account.identifier])?;
            tx.execute(
                "INSERT INTO repayment_allocations VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    id,
                    payment,
                    a.debt_id,
                    a.amount,
                    a.remaining_after,
                    position as i64
                ],
            )?;
            tx.execute(
                "UPDATE debts SET remaining=?2,status=?3 WHERE id=?1",
                params![
                    a.debt_id,
                    a.remaining_after,
                    if a.remaining_after == Some(0) {
                        "paid"
                    } else {
                        "partial"
                    }
                ],
            )?;
        }
        db::audit_tx(
            tx,
            "customer_repayment_recorded",
            "customer_repayment",
            Some(&id),
            json!({"customerId":client.id,"amount":input.amount,"account":account,"allocations":preview.allocations}),
        )?;
        Ok(CustomerRepayment {
            id,
            customer_id: client.id,
            customer_name: client.name,
            customer_phone: client.phone,
            amount: input.amount,
            account_snapshot: account,
            paid_at: input.paid_at.clone(),
            note,
            created_at: created,
            legacy: false,
            allocations: preview.allocations,
        })
    })
}

pub fn repayments(
    c: &Connection,
    filters: Option<&ReportFilters>,
) -> AppResult<Vec<CustomerRepayment>> {
    let mut stmt = c.prepare("SELECT id,customer_id,customer_name,customer_phone,amount,account_json,paid_at,note,created_at FROM customer_repayments")?;
    let raw = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Money>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, String>(8)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut result = Vec::new();
    for (
        id,
        customer_id,
        customer_name,
        customer_phone,
        amount,
        account_json,
        paid_at,
        note,
        created_at,
    ) in raw
    {
        let mut allocations = c.prepare("SELECT a.debt_id,d.issued_at,a.amount,a.remaining_after FROM repayment_allocations a JOIN debts d ON d.id=a.debt_id WHERE a.repayment_id=?1 ORDER BY a.position")?;
        let allocations = allocations
            .query_map([&id], |r| {
                Ok(RepaymentAllocation {
                    debt_id: r.get(0)?,
                    issued_at: r.get(1)?,
                    amount: r.get(2)?,
                    remaining_after: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        result.push(CustomerRepayment {
            id,
            customer_id,
            customer_name,
            customer_phone,
            amount,
            account_snapshot: serde_json::from_str(&account_json)?,
            paid_at,
            note,
            created_at,
            legacy: false,
            allocations,
        });
    }
    let mut stmt = c.prepare("SELECT p.id,p.debt_id,p.amount,p.account,p.paid_at,p.note,p.created_at,p.account_id,p.account_name,p.account_identifier,d.customer_id,d.customer_name,d.phone,d.issued_at FROM debt_payments p JOIN debts d ON d.id=p.debt_id WHERE NOT EXISTS(SELECT 1 FROM repayment_allocations a WHERE a.payment_id=p.id)")?;
    for row in stmt.query_map([], |r| {
        Ok(CustomerRepayment {
            id: r.get(0)?,
            customer_id: r.get(10)?,
            customer_name: r.get(11)?,
            customer_phone: r.get(12)?,
            amount: r.get(2)?,
            account_snapshot: AccountSnapshot {
                account_id: r.get(7)?,
                provider: r.get(3)?,
                name: r.get(8)?,
                identifier: r.get(9)?,
            },
            paid_at: r.get(4)?,
            note: r.get(5)?,
            created_at: r.get(6)?,
            legacy: true,
            allocations: vec![RepaymentAllocation {
                debt_id: r.get(1)?,
                issued_at: r.get(13)?,
                amount: r.get(2)?,
                remaining_after: None,
            }],
        })
    })? {
        result.push(row?);
    }
    if let Some(f) = filters {
        result.retain(|p| {
            f.from.as_ref().is_none_or(|d| p.paid_at >= *d)
                && f.to.as_ref().is_none_or(|d| p.paid_at <= *d)
        });
    }
    result.sort_by(|a, b| {
        (&b.paid_at, &b.created_at, &b.id).cmp(&(&a.paid_at, &a.created_at, &a.id))
    });
    Ok(result)
}

#[cfg(test)]
#[path = "debt_client_tests.rs"]
mod tests;
