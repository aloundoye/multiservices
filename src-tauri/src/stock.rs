use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    db,
    domain::{clean_optional, clean_required},
    error::{AppError, AppResult},
    models::*,
};

// Amounts and quantities cross the JavaScript boundary as exact integers.
const MAX_INTEGER: i64 = 9_007_199_254_740_991;

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

fn whole(value: i64, positive: bool) -> AppResult<i64> {
    if value < i64::from(positive) || value > MAX_INTEGER {
        return Err(invalid(
            "Montant ou quantité invalide : utilisez un entier positif dans la limite autorisée.",
        ));
    }
    Ok(value)
}

fn add(a: i64, b: i64) -> AppResult<i64> {
    whole(
        a.checked_add(b)
            .ok_or_else(|| invalid("Limite de calcul dépassée."))?,
        false,
    )
}

// Keep the result with the request in the same transaction: a retry after a lost
// response returns the original result, even if the product has since changed.
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
    let previous: Option<(String, String, String)> = tx
        .query_row(
            "SELECT action, input_json, result_json FROM stock_requests WHERE id=?1",
            [request_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((previous_action, previous_input, result)) = previous {
        if previous_action != action || previous_input != payload {
            return Err(invalid("Cet identifiant a déjà été utilisé pour une autre opération. Fermez puis rouvrez le formulaire."));
        }
        return Ok(serde_json::from_str(&result)?);
    }
    let result = work(&tx)?;
    tx.execute(
        "INSERT INTO stock_requests VALUES (?1,?2,?3,?4)",
        params![request_id, action, payload, serde_json::to_string(&result)?],
    )?;
    tx.commit()?;
    Ok(result)
}

fn map_product(r: &rusqlite::Row<'_>) -> rusqlite::Result<Product> {
    Ok(Product {
        id: r.get(0)?,
        name: r.get(1)?,
        price: r.get(2)?,
        stock: r.get(3)?,
        active: r.get(4)?,
    })
}

pub fn products(connection: &Connection) -> AppResult<Vec<Product>> {
    let mut stmt = connection.prepare("SELECT id,name,price,stock,active FROM products ORDER BY active DESC, name COLLATE NOCASE, id")?;
    let result = stmt
        .query_map([], map_product)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result)
}

fn product(connection: &Connection, id: &str, require_active: bool) -> AppResult<Product> {
    let product = connection
        .query_row(
            "SELECT id,name,price,stock,active FROM products WHERE id=?1",
            [id],
            map_product,
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    if require_active && !product.active {
        return Err(invalid("Ce produit est archivé."));
    }
    Ok(product)
}

struct Movement<'a> {
    operation_id: Option<&'a str>,
    kind: &'a str,
    delta: i64,
    reason: Option<&'a str>,
    date: &'a str,
}

fn move_stock(tx: &Transaction<'_>, product: &Product, movement: Movement<'_>) -> AppResult<()> {
    let after = product
        .stock
        .checked_add(movement.delta)
        .ok_or_else(|| invalid("Limite de stock dépassée."))?;
    if after < 0 {
        return Err(invalid(&format!(
            "Stock insuffisant pour {} ({} disponible).",
            product.name, product.stock
        )));
    }
    whole(after, false)?;
    // Cancelling an old sale may return an archived product to the catalogue.
    tx.execute(
        "UPDATE products SET stock=?1, active=CASE WHEN ?1>0 THEN 1 ELSE active END WHERE id=?2",
        params![after, product.id],
    )?;
    tx.execute(
        "INSERT INTO stock_movements (id,product_id,product_name,operation_id,kind,quantity,balance_after,reason,occurred_at,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![Uuid::new_v4().to_string(), product.id, product.name, movement.operation_id, movement.kind,
            movement.delta, after, movement.reason, movement.date, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn create_product(
    connection: &mut Connection,
    input: CreateProductInput,
) -> AppResult<Product> {
    once(
        connection,
        &input.request_id,
        "product_created",
        &input,
        |tx| {
            let name = clean_required(&input.name, "Le nom du produit", 1)?;
            whole(input.price, true)?;
            whole(input.initial_stock, false)?;
            let id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO products (id,name,price,stock,created_at) VALUES (?1,?2,?3,0,?4)",
                params![id, name, input.price, Utc::now().to_rfc3339()],
            )?;
            let initial = product(tx, &id, true)?;
            move_stock(
                tx,
                &initial,
                Movement {
                    operation_id: None,
                    kind: "initial",
                    delta: input.initial_stock,
                    reason: Some("Marchandises déjà détenues"),
                    date: &Utc::now().date_naive().to_string(),
                },
            )?;
            db::audit_tx(
                tx,
                "product_created",
                "product",
                Some(&id),
                json!({"name": name, "price": input.price, "initialStock": input.initial_stock}),
            )?;
            product(tx, &id, true)
        },
    )
}

pub fn update_product(
    connection: &mut Connection,
    input: UpdateProductInput,
) -> AppResult<Product> {
    once(
        connection,
        &input.request_id,
        "product_updated",
        &input,
        |tx| {
            let before = product(tx, &input.product_id, true)?;
            let name = clean_required(&input.name, "Le nom du produit", 1)?;
            whole(input.price, true)?;
            tx.execute(
                "UPDATE products SET name=?1,price=?2 WHERE id=?3",
                params![name, input.price, input.product_id],
            )?;
            db::audit_tx(
                tx,
                "product_updated",
                "product",
                Some(&input.product_id),
                json!({"before": before, "name": name, "price": input.price}),
            )?;
            product(tx, &input.product_id, true)
        },
    )
}

pub fn archive_product(
    connection: &mut Connection,
    input: ArchiveProductInput,
) -> AppResult<Product> {
    once(
        connection,
        &input.request_id,
        "product_archived",
        &input,
        |tx| {
            let current = product(tx, &input.product_id, true)?;
            if current.stock != 0 {
                return Err(invalid("Le stock doit être nul pour archiver un produit."));
            }
            tx.execute(
                "UPDATE products SET active=0 WHERE id=?1",
                [&input.product_id],
            )?;
            db::audit_tx(
                tx,
                "product_archived",
                "product",
                Some(&input.product_id),
                json!({"name": current.name}),
            )?;
            product(tx, &input.product_id, false)
        },
    )
}

pub fn adjust_stock(connection: &mut Connection, input: AdjustStockInput) -> AppResult<Product> {
    once(
        connection,
        &input.request_id,
        "stock_adjusted",
        &input,
        |tx| {
            let current = product(tx, &input.product_id, true)?;
            whole(input.quantity, false)?;
            let reason = clean_required(&input.reason, "Le motif", 3)?;
            if current.stock == input.quantity {
                return Err(invalid(
                    "La quantité comptée est identique au stock actuel.",
                ));
            }
            move_stock(
                tx,
                &current,
                Movement {
                    operation_id: None,
                    kind: "adjustment",
                    delta: input.quantity - current.stock,
                    reason: Some(&reason),
                    date: &Utc::now().date_naive().to_string(),
                },
            )?;
            db::audit_tx(
                tx,
                "stock_adjusted",
                "product",
                Some(&input.product_id),
                json!({"before": current.stock, "after": input.quantity, "reason": reason}),
            )?;
            product(tx, &input.product_id, true)
        },
    )
}

pub fn journal_link(
    connection: &Connection,
    entry_id: &str,
) -> AppResult<Option<ProductOperationLink>> {
    Ok(connection
        .query_row(
            "SELECT o.id,o.kind FROM product_operations o WHERE o.journal_entry_id=?1
         OR o.journal_entry_id=(SELECT reverses_id FROM journal_entries WHERE id=?1)",
            [entry_id],
            |r| {
                Ok(ProductOperationLink {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                })
            },
        )
        .optional()?)
}

fn operation(connection: &Connection, id: &str) -> AppResult<ProductOperation> {
    let mut result = connection.query_row(
        "SELECT o.id,o.kind,o.journal_entry_id,o.cancelled_at,o.cancellation_reason,
         j.account_id,j.payment_account,j.account_name,j.account_identifier,j.occurred_at,j.posted_at,j.amount,j.note
         FROM product_operations o JOIN journal_entries j ON j.id=o.journal_entry_id WHERE o.id=?1", [id], |r| {
            Ok(ProductOperation { id: r.get(0)?, kind: r.get(1)?, journal_entry_id: r.get(2)?,
                cancelled_at: r.get(3)?, cancellation_reason: r.get(4)?,
                account_snapshot: AccountSnapshot { account_id: r.get(5)?, provider: r.get(6)?, name: r.get(7)?, identifier: r.get(8)? },
                occurred_at: r.get(9)?, created_at: r.get(10)?, amount: r.get(11)?, note: r.get(12)?, lines: vec![] })
        }).optional()?.ok_or(AppError::NotFound)?;
    let mut stmt = connection.prepare("SELECT product_id,product_name,quantity,unit_price,total FROM product_operation_lines WHERE operation_id=?1 ORDER BY rowid")?;
    result.lines = stmt
        .query_map([id], |r| {
            Ok(ProductOperationLine {
                product_id: r.get(0)?,
                product_name: r.get(1)?,
                quantity: r.get(2)?,
                unit_price: r.get(3)?,
                total: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result)
}

pub fn operations(connection: &Connection) -> AppResult<Vec<ProductOperation>> {
    let mut stmt = connection.prepare("SELECT o.id FROM product_operations o JOIN journal_entries j ON j.id=o.journal_entry_id ORDER BY j.occurred_at DESC,j.posted_at DESC")?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    ids.iter().map(|id| operation(connection, id)).collect()
}

struct Posting<'a> {
    id: &'a str,
    kind: &'a str,
    account_id: &'a str,
    date: &'a str,
    note: Option<String>,
    lines: Vec<ProductOperationLine>,
}

fn post(tx: &Transaction<'_>, posting: Posting<'_>) -> AppResult<ProductOperation> {
    let amount = posting
        .lines
        .iter()
        .try_fold(0, |sum, line| add(sum, line.total))?;
    let description = posting
        .lines
        .iter()
        .map(|l| format!("{} × {}", l.quantity, l.product_name))
        .collect::<Vec<_>>()
        .join(", ");
    let note = match clean_optional(posting.note) {
        Some(note) => format!("{description} — {note}"),
        None => description,
    };
    let entry = db::create_journal_entry_tx(
        tx,
        CreateJournalEntryInput {
            entry_type: if posting.kind == "sale" {
                "sale"
            } else {
                "purchase"
            }
            .into(),
            amount,
            account_id: posting.account_id.into(),
            occurred_at: posting.date.into(),
            reference: Some(format!(
                "{} {}",
                if posting.kind == "sale" {
                    "Vente"
                } else {
                    "Réapprovisionnement"
                },
                posting.id
            )),
            note: Some(note),
        },
    )?;
    check_capital(tx)?;
    tx.execute(
        "INSERT INTO product_operations (id,kind,journal_entry_id) VALUES (?1,?2,?3)",
        params![posting.id, posting.kind, entry.id],
    )?;
    for line in &posting.lines {
        tx.execute(
            "INSERT INTO product_operation_lines VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                posting.id,
                line.product_id,
                line.product_name,
                line.quantity,
                line.unit_price,
                line.total
            ],
        )?;
        let current = product(tx, &line.product_id, true)?;
        move_stock(
            tx,
            &current,
            Movement {
                operation_id: Some(posting.id),
                kind: posting.kind,
                delta: if posting.kind == "sale" {
                    -line.quantity
                } else {
                    line.quantity
                },
                reason: None,
                date: posting.date,
            },
        )?;
    }
    db::audit_tx(
        tx,
        if posting.kind == "sale" {
            "product_sale_created"
        } else {
            "stock_received"
        },
        "product_operation",
        Some(posting.id),
        json!({"amount": amount, "journalEntryId": entry.id, "lines": posting.lines}),
    )?;
    operation(tx, posting.id)
}

pub(crate) fn check_capital(connection: &Connection) -> AppResult<()> {
    let capital = db::get_dashboard(connection)?.expected_capital;
    if !(-MAX_INTEGER..=MAX_INTEGER).contains(&capital) {
        return Err(invalid("Le capital attendu dépasse la limite autorisée."));
    }
    Ok(())
}

pub fn create_sale(
    connection: &mut Connection,
    input: CreateSaleInput,
) -> AppResult<ProductOperation> {
    once(
        connection,
        &input.request_id,
        "product_sale_created",
        &input,
        |tx| {
            if input.lines.is_empty() {
                return Err(invalid("Ajoutez au moins un produit à la vente."));
            }
            let mut seen = HashSet::new();
            let mut lines = Vec::new();
            for line in &input.lines {
                if !seen.insert(&line.product_id) {
                    return Err(invalid(
                        "Regroupez les quantités d’un même produit sur une seule ligne.",
                    ));
                }
                whole(line.quantity, true)?;
                whole(line.unit_price, true)?;
                let current = product(tx, &line.product_id, true)?;
                let total = whole(
                    line.quantity
                        .checked_mul(line.unit_price)
                        .ok_or_else(|| invalid("Le total dépasse la limite autorisée."))?,
                    true,
                )?;
                lines.push(ProductOperationLine {
                    product_id: current.id,
                    product_name: current.name,
                    quantity: line.quantity,
                    unit_price: Some(line.unit_price),
                    total,
                });
            }
            post(
                tx,
                Posting {
                    id: &input.request_id,
                    kind: "sale",
                    account_id: &input.account_id,
                    date: &input.occurred_at,
                    note: input.note.clone(),
                    lines,
                },
            )
        },
    )
}

pub fn receive_stock(
    connection: &mut Connection,
    input: ReceiveStockInput,
) -> AppResult<ProductOperation> {
    once(
        connection,
        &input.request_id,
        "stock_received",
        &input,
        |tx| {
            whole(input.quantity, true)?;
            whole(input.amount, true)?;
            let current = product(tx, &input.product_id, true)?;
            post(
                tx,
                Posting {
                    id: &input.request_id,
                    kind: "receipt",
                    account_id: &input.account_id,
                    date: &input.occurred_at,
                    note: input.note.clone(),
                    lines: vec![ProductOperationLine {
                        product_id: current.id,
                        product_name: current.name,
                        quantity: input.quantity,
                        unit_price: None,
                        total: input.amount,
                    }],
                },
            )
        },
    )
}

// Called by the journal reversal itself, so no caller can reverse money alone.
pub(crate) fn reverse_stock_tx(
    tx: &Transaction<'_>,
    journal_id: &str,
    reason: &str,
) -> AppResult<()> {
    let Some(link) = journal_link(tx, journal_id)? else {
        return Ok(());
    };
    let original = operation(tx, &link.id)?;
    if original.cancelled_at.is_some() {
        return Err(invalid("Cette opération est déjà annulée."));
    }
    for line in &original.lines {
        let current = product(tx, &line.product_id, false)?;
        move_stock(
            tx,
            &current,
            Movement {
                operation_id: Some(&original.id),
                kind: "cancellation",
                delta: if original.kind == "sale" {
                    line.quantity
                } else {
                    -line.quantity
                },
                reason: Some(reason),
                date: &Utc::now().date_naive().to_string(),
            },
        )?;
    }
    tx.execute(
        "UPDATE product_operations SET cancelled_at=?1,cancellation_reason=?2 WHERE id=?3",
        params![Utc::now().to_rfc3339(), reason, original.id],
    )?;
    db::audit_tx(
        tx,
        "product_operation_cancelled",
        "product_operation",
        Some(&original.id),
        json!({"reason": reason, "kind": original.kind}),
    )?;
    Ok(())
}

pub fn cancel_operation(
    connection: &mut Connection,
    input: CancelProductOperationInput,
) -> AppResult<ProductOperation> {
    once(
        connection,
        &input.request_id,
        "product_operation_cancelled",
        &input,
        |tx| {
            let original = operation(tx, &input.operation_id)?;
            db::reverse_journal_entry_tx(
                tx,
                ReverseEntryInput {
                    entry_id: original.journal_entry_id,
                    reason: input.reason.clone(),
                },
            )?;
            operation(tx, &input.operation_id)
        },
    )
}

pub fn movements(
    connection: &Connection,
    product_id: Option<&str>,
) -> AppResult<Vec<StockMovement>> {
    let mut stmt = connection.prepare("SELECT id,product_id,product_name,operation_id,kind,quantity,balance_after,reason,occurred_at,created_at FROM stock_movements WHERE (?1 IS NULL OR product_id=?1) ORDER BY created_at DESC,rowid DESC")?;
    let result = stmt
        .query_map([product_id], |r| {
            Ok(StockMovement {
                id: r.get(0)?,
                product_id: r.get(1)?,
                product_name: r.get(2)?,
                operation_id: r.get(3)?,
                kind: r.get(4)?,
                quantity: r.get(5)?,
                balance_after: r.get(6)?,
                reason: r.get(7)?,
                occurred_at: r.get(8)?,
                created_at: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(result)
}

#[cfg(test)]
#[path = "stock_tests.rs"]
mod tests;
