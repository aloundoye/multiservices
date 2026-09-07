use crate::{
    db,
    domain::{
        clean_optional, clean_required, validate_balances, validate_initial_allocation,
        validate_payment_account,
    },
    error::{AppError, AppResult},
    models::*,
};
use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

pub fn list(connection: &Connection) -> AppResult<Vec<Account>> {
    let mut statement = connection.prepare(
        "SELECT a.id, a.provider, a.name, a.identifier, a.active,
         (SELECT b.amount FROM inventory_account_balances b JOIN inventories i ON i.id=b.inventory_id
          WHERE b.account_id=a.id ORDER BY i.closed_at DESC, i.rowid DESC LIMIT 1),
         (SELECT i.closed_at FROM inventory_account_balances b JOIN inventories i ON i.id=b.inventory_id
          WHERE b.account_id=a.id ORDER BY i.closed_at DESC, i.rowid DESC LIMIT 1)
         FROM accounts a ORDER BY CASE a.provider WHEN 'orange_money' THEN 0 WHEN 'wave' THEN 1 WHEN 'djamo' THEN 2 ELSE 3 END, a.name_key"
    )?;
    let rows = statement
        .query_map([], |r| {
            Ok(Account {
                snapshot: AccountSnapshot {
                    account_id: r.get(0)?,
                    provider: r.get(1)?,
                    name: r.get(2)?,
                    identifier: r.get(3)?,
                },
                active: r.get(4)?,
                last_balance: r.get(5)?,
                last_measured_at: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(connection: &Connection, id: &str, require_active: bool) -> AppResult<Account> {
    let account = list(connection)?
        .into_iter()
        .find(|a| a.snapshot.account_id == id)
        .ok_or(AppError::NotFound)?;
    if require_active && !account.active {
        return Err(AppError::Validation("Ce compte est archivé.".into()));
    }
    Ok(account)
}

fn validate_name(
    connection: &Connection,
    provider: &str,
    name: &str,
    except: &str,
) -> AppResult<String> {
    let name = clean_required(name, "Le nom du compte", 1)?;
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE provider=?1 AND name_key=?2 AND id!=?3)",
        params![provider, name.to_lowercase(), except],
        |r| r.get(0),
    )?;
    if exists {
        return Err(AppError::Validation(
            "Ce nom existe déjà pour ce service (y compris les comptes archivés).".into(),
        ));
    }
    Ok(name)
}

pub fn insert(
    tx: &Transaction<'_>,
    input: CreateAccountInput,
    allow_cash: bool,
) -> AppResult<AccountSnapshot> {
    validate_payment_account(&input.provider)?;
    if input.provider == "cash" && !allow_cash {
        return Err(AppError::Validation(
            "La boutique possède une seule caisse espèces.".into(),
        ));
    }
    let name = validate_name(tx, &input.provider, &input.name, "")?;
    let account = AccountSnapshot {
        account_id: Uuid::new_v4().to_string(),
        provider: input.provider,
        name,
        identifier: clean_optional(input.identifier),
    };
    tx.execute("INSERT INTO accounts(id,provider,name,name_key,identifier,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![account.account_id,account.provider,account.name,account.name.to_lowercase(),account.identifier,Utc::now().to_rfc3339()])?;
    db::audit_tx(
        tx,
        "account_created",
        "account",
        Some(&account.account_id),
        json!(account),
    )?;
    Ok(account)
}

pub fn create(connection: &mut Connection, input: CreateAccountInput) -> AppResult<Account> {
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let account = insert(&tx, input, false)?;
    tx.commit()?;
    get(connection, &account.account_id, false)
}

pub fn update(connection: &mut Connection, input: UpdateAccountInput) -> AppResult<Account> {
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let old = get(&tx, &input.account_id, false)?;
    let name = validate_name(&tx, &old.snapshot.provider, &input.name, &input.account_id)?;
    let identifier = clean_optional(input.identifier);
    tx.execute(
        "UPDATE accounts SET name=?1,name_key=?2,identifier=?3 WHERE id=?4",
        params![name, name.to_lowercase(), identifier, input.account_id],
    )?;
    db::audit_tx(
        &tx,
        "account_updated",
        "account",
        Some(&input.account_id),
        json!({"before":old.snapshot,"name":name,"identifier":identifier}),
    )?;
    tx.commit()?;
    get(connection, &input.account_id, false)
}

pub fn set_active(connection: &mut Connection, id: &str, active: bool) -> AppResult<Account> {
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let account = get(&tx, id, false)?;
    if !active && account.snapshot.provider == "cash" {
        return Err(AppError::Validation(
            "La caisse espèces ne peut pas être archivée.".into(),
        ));
    }
    if !active && account.active {
        if account.last_balance.is_some_and(|v| v != 0) {
            return Err(AppError::Validation(
                "Clôturez d’abord un inventaire avec un solde nul sur ce compte.".into(),
            ));
        }
        let since = account.last_measured_at.as_deref().unwrap_or("");
        let pending: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM journal_entries WHERE account_id=?1 AND posted_at>=?2
             UNION ALL SELECT 1 FROM debts WHERE account_id=?1 AND created_at>=?2
             UNION ALL SELECT 1 FROM debt_payments WHERE account_id=?1 AND created_at>=?2)",
            params![id, since],
            |r| r.get(0),
        )?;
        if pending {
            return Err(AppError::Validation("Une opération a été enregistrée depuis le dernier relevé : faites un inventaire avant l’archivage.".into()));
        }
    }
    if active != account.active {
        tx.execute(
            "UPDATE accounts SET active=?1 WHERE id=?2",
            params![active, id],
        )?;
        db::audit_tx(
            &tx,
            if active {
                "account_reactivated"
            } else {
                "account_archived"
            },
            "account",
            Some(id),
            json!(account.snapshot),
        )?;
    }
    tx.commit()?;
    get(connection, id, false)
}

pub fn totals<'a>(
    values: impl IntoIterator<Item = (&'a str, Money)>,
) -> AppResult<AccountBalances> {
    let mut result = AccountBalances {
        orange_money: 0,
        wave: 0,
        djamo: 0,
        cash: 0,
    };
    for (provider, amount) in values {
        if !(0..=9_007_199_254_740_991).contains(&amount) {
            return Err(AppError::Validation(
                "Solde entier positif ou nul requis, dans la limite autorisée.".into(),
            ));
        }
        let slot = match provider {
            "orange_money" => &mut result.orange_money,
            "wave" => &mut result.wave,
            "djamo" => &mut result.djamo,
            "cash" => &mut result.cash,
            _ => return Err(AppError::Validation("Service inconnu.".into())),
        };
        *slot = slot
            .checked_add(amount)
            .ok_or_else(|| AppError::Validation("Total trop élevé.".into()))?;
    }
    if validate_balances(&result)? > 9_007_199_254_740_991 {
        return Err(AppError::Validation("Total trop élevé.".into()));
    }
    Ok(result)
}

pub fn opening_totals(inputs: &[OpeningAccount]) -> AppResult<AccountBalances> {
    let mut names = HashSet::new();
    for a in inputs {
        let name = clean_required(&a.name, "Le nom du compte", 1)?;
        if !names.insert((a.provider.as_str(), name.to_lowercase())) {
            return Err(AppError::Validation(
                "Deux comptes du même service portent le même nom.".into(),
            ));
        }
    }
    if inputs.iter().filter(|a| a.provider == "cash").count() != 1 {
        return Err(AppError::Validation(
            "La répartition doit contenir exactement une caisse espèces.".into(),
        ));
    }
    totals(inputs.iter().map(|a| (a.provider.as_str(), a.amount)))
}

pub fn validate_opening(input: &SetupInput) -> AppResult<AccountBalances> {
    let totals = opening_totals(&input.accounts)?;
    validate_initial_allocation(input.initial_capital, &totals)?;
    Ok(totals)
}

pub fn preview(
    connection: &Connection,
    inputs: &[AccountBalanceInput],
) -> AppResult<(AccountBalances, Vec<AccountBalanceSnapshot>)> {
    let active: Vec<_> = list(connection)?.into_iter().filter(|a| a.active).collect();
    if inputs.len() != active.len() {
        return Err(AppError::Validation(
            "Saisissez exactement un solde pour chaque compte actif.".into(),
        ));
    }
    let mut seen = HashSet::new();
    let mut details = Vec::new();
    for input in inputs {
        if !seen.insert(&input.account_id) {
            return Err(AppError::Validation(
                "Un compte apparaît plusieurs fois.".into(),
            ));
        }
        let account = active
            .iter()
            .find(|a| a.snapshot.account_id == input.account_id)
            .ok_or_else(|| {
                AppError::Validation("Compte inconnu ou archivé dans l’inventaire.".into())
            })?;
        details.push(AccountBalanceSnapshot {
            account: account.snapshot.clone(),
            amount: input.amount,
            previous_amount: account.last_balance,
            delta: account
                .last_balance
                .and_then(|v| input.amount.checked_sub(v)),
            legacy: false,
        });
    }
    let totals = totals(
        details
            .iter()
            .map(|a| (a.account.provider.as_str(), a.amount)),
    )?;
    Ok((totals, details))
}

pub fn save_balances(
    tx: &Transaction<'_>,
    inventory_id: &str,
    details: &[AccountBalanceSnapshot],
) -> AppResult<()> {
    for d in details {
        tx.execute("INSERT INTO inventory_account_balances(inventory_id,account_id,provider,account_name,account_identifier,amount,legacy) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![inventory_id,d.account.account_id,d.account.provider,d.account.name,d.account.identifier,d.amount,d.legacy])?;
    }
    Ok(())
}

pub fn inventory_balances(
    connection: &Connection,
    inventory_id: &str,
) -> AppResult<Vec<AccountBalanceSnapshot>> {
    let mut statement=connection.prepare(
        "SELECT b.account_id,b.provider,b.account_name,b.account_identifier,b.amount,b.legacy,
         (SELECT p.amount FROM inventory_account_balances p JOIN inventories pi ON pi.id=p.inventory_id
          WHERE p.account_id=b.account_id AND (pi.closed_at<i.closed_at OR (pi.closed_at=i.closed_at AND pi.rowid<i.rowid))
          ORDER BY pi.closed_at DESC,pi.rowid DESC LIMIT 1)
         FROM inventory_account_balances b JOIN inventories i ON i.id=b.inventory_id
         WHERE b.inventory_id=?1 ORDER BY b.provider,b.account_name")?;
    let values = statement
        .query_map([inventory_id], |r| {
            let amount: Money = r.get(4)?;
            let previous_amount: Option<Money> = r.get(6)?;
            Ok(AccountBalanceSnapshot {
                account: AccountSnapshot {
                    account_id: r.get(0)?,
                    provider: r.get(1)?,
                    name: r.get(2)?,
                    identifier: r.get(3)?,
                },
                amount,
                previous_amount,
                delta: previous_amount.map(|v| amount - v),
                legacy: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(values)
}
