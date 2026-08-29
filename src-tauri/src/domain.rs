use crate::{
    error::{AppError, AppResult},
    models::{AccountBalances, Money},
};

pub const POSITIVE_ENTRY_TYPES: &[&str] = &["sale", "commission", "capital_contribution"];
pub const NEGATIVE_ENTRY_TYPES: &[&str] = &["purchase", "expense", "capital_withdrawal"];
pub const PAYMENT_ACCOUNTS: &[&str] = &["cash", "orange_money", "wave", "djamo"];
pub const DEBT_PROVIDERS: &[&str] = &["orange_money", "wave"];
pub const VARIANCE_CATEGORIES: &[&str] = &[
    "commission_mobile",
    "surplus_caisse",
    "manquant_caisse",
    "erreur_saisie",
    "autre",
];

pub fn validate_balances(balances: &AccountBalances) -> AppResult<Money> {
    if !balances.all_non_negative() {
        return Err(AppError::Validation(
            "Les soldes ne peuvent pas être négatifs.".into(),
        ));
    }
    balances.liquidity().ok_or_else(|| {
        AppError::Validation("Le total dépasse la capacité de calcul autorisée.".into())
    })
}

pub fn validate_initial_allocation(
    initial_capital: Money,
    balances: &AccountBalances,
) -> AppResult<()> {
    if initial_capital <= 0 {
        return Err(AppError::Validation(
            "Le capital initial doit être supérieur à zéro.".into(),
        ));
    }
    let liquidity = validate_balances(balances)?;
    if liquidity != initial_capital {
        return Err(AppError::Validation(format!(
            "La répartition doit être exactement égale au capital initial (écart: {} FCFA).",
            liquidity - initial_capital
        )));
    }
    Ok(())
}

pub fn signed_journal_amount(entry_type: &str, amount: Money) -> AppResult<Money> {
    if amount <= 0 {
        return Err(AppError::Validation(
            "Le montant doit être supérieur à zéro.".into(),
        ));
    }
    if POSITIVE_ENTRY_TYPES.contains(&entry_type) {
        Ok(amount)
    } else if NEGATIVE_ENTRY_TYPES.contains(&entry_type) {
        amount
            .checked_neg()
            .ok_or_else(|| AppError::Validation("Montant invalide.".into()))
    } else {
        Err(AppError::Validation("Type d’écriture non reconnu.".into()))
    }
}

pub fn validate_payment_account(value: &str) -> AppResult<()> {
    if PAYMENT_ACCOUNTS.contains(&value) {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Compte de paiement non reconnu.".into(),
        ))
    }
}

pub fn validate_debt_provider(value: &str) -> AppResult<()> {
    if DEBT_PROVIDERS.contains(&value) {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Une dette doit provenir d’Orange Money ou de Wave.".into(),
        ))
    }
}

pub fn validate_variance_explanation(
    variance: Money,
    category: Option<&str>,
    note: Option<&str>,
) -> AppResult<()> {
    if variance == 0 {
        return Ok(());
    }
    let category = category.unwrap_or_default();
    if !VARIANCE_CATEGORIES.contains(&category) {
        return Err(AppError::Validation(
            "Choisissez une catégorie pour expliquer l’écart.".into(),
        ));
    }
    if note.unwrap_or_default().trim().len() < 3 {
        return Err(AppError::Validation(
            "Ajoutez une explication d’au moins trois caractères.".into(),
        ));
    }
    Ok(())
}

pub fn validate_pin(pin: &str) -> AppResult<()> {
    if !(4..=12).contains(&pin.len()) || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Validation(
            "Le PIN doit contenir entre 4 et 12 chiffres.".into(),
        ));
    }
    Ok(())
}

pub fn validate_recovery_password(password: &str) -> AppResult<()> {
    if password.chars().count() < 12 {
        return Err(AppError::Validation(
            "Le mot de passe de récupération doit contenir au moins 12 caractères.".into(),
        ));
    }
    Ok(())
}

pub fn clean_required(value: &str, label: &str, min: usize) -> AppResult<String> {
    let value = value.trim();
    if value.chars().count() < min {
        return Err(AppError::Validation(format!(
            "{label} doit contenir au moins {min} caractères."
        )));
    }
    Ok(value.to_string())
}

pub fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_opening_allocation_is_required() {
        let valid = AccountBalances {
            orange_money: 1_500_000,
            wave: 1_200_000,
            djamo: 800_000,
            cash: 1_500_000,
        };
        assert!(validate_initial_allocation(5_000_000, &valid).is_ok());

        let invalid = AccountBalances {
            cash: 1_499_999,
            ..valid
        };
        assert!(validate_initial_allocation(5_000_000, &invalid).is_err());
    }

    #[test]
    fn journal_signs_follow_cash_based_rules() {
        assert_eq!(signed_journal_amount("sale", 100_000).unwrap(), 100_000);
        assert_eq!(signed_journal_amount("expense", 30_000).unwrap(), -30_000);
        assert_eq!(100_000 - 30_000 + 5_000_000, 5_070_000);
    }

    #[test]
    fn non_zero_variance_needs_a_reason() {
        assert!(validate_variance_explanation(1, None, None).is_err());
        assert!(validate_variance_explanation(
            -10_000,
            Some("manquant_caisse"),
            Some("Écart constaté à la fermeture")
        )
        .is_ok());
    }
}
