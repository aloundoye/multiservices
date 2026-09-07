use serde::{Deserialize, Serialize};

pub type Money = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub initialized: bool,
    pub unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupInput {
    pub business_name: String,
    pub pin: String,
    pub recovery_password: String,
    pub initial_capital: Money,
    pub accounts: Vec<OpeningAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessSettings {
    pub business_name: String,
    pub currency: String,
    pub timezone: String,
    pub inventory_interval_minutes: i64,
    pub auto_lock_minutes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub business_name: String,
    pub inventory_interval_minutes: i64,
    pub auto_lock_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalances {
    pub orange_money: Money,
    pub wave: Money,
    pub djamo: Money,
    pub cash: Money,
}

impl AccountBalances {
    pub fn liquidity(&self) -> Option<Money> {
        self.orange_money
            .checked_add(self.wave)?
            .checked_add(self.djamo)?
            .checked_add(self.cash)
    }

    pub fn all_non_negative(&self) -> bool {
        self.orange_money >= 0 && self.wave >= 0 && self.djamo >= 0 && self.cash >= 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub account_balances: Vec<AccountBalanceSnapshot>,
    pub id: String,
    pub kind: String,
    pub closed_at: String,
    pub balances: AccountBalances,
    pub receivables: Money,
    pub liquidity: Money,
    pub expected_total: Money,
    pub actual_total: Money,
    pub variance: Money,
    pub variance_category: Option<String>,
    pub variance_note: Option<String>,
    pub delta: AccountBalances,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryPreviewInput {
    pub balances: Vec<AccountBalanceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryPreview {
    pub account_balances: Vec<AccountBalanceSnapshot>,
    pub balances: AccountBalances,
    pub previous_balances: AccountBalances,
    pub delta: AccountBalances,
    pub receivables: Money,
    pub liquidity: Money,
    pub expected_total: Money,
    pub actual_total: Money,
    pub variance: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseInventoryInput {
    pub balances: Vec<AccountBalanceInput>,
    pub variance_category: Option<String>,
    pub variance_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseInventoryResult {
    pub inventory: Inventory,
    pub backup: Option<BackupInfo>,
    pub backup_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCorrectionInput {
    pub inventory_id: String,
    pub amount: Money,
    pub direction: String,
    pub account_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub account_snapshot: AccountSnapshot,
    pub id: String,
    pub entry_type: String,
    pub amount: Money,
    pub signed_amount: Money,
    pub payment_account: String,
    pub occurred_at: String,
    pub posted_at: String,
    pub reference: Option<String>,
    pub note: Option<String>,
    pub reverses_id: Option<String>,
    pub reversed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJournalEntryInput {
    pub entry_type: String,
    pub amount: Money,
    pub account_id: String,
    pub occurred_at: String,
    pub reference: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseEntryInput {
    pub entry_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Debt {
    pub account_snapshot: AccountSnapshot,
    pub id: String,
    pub customer_name: String,
    pub phone: String,
    pub provider: String,
    pub principal: Money,
    pub remaining: Money,
    pub issued_at: String,
    pub due_date: Option<String>,
    pub note: Option<String>,
    pub status: String,
    pub created_at: String,
    pub payments: Vec<DebtPayment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtPayment {
    pub account_snapshot: AccountSnapshot,
    pub id: String,
    pub debt_id: String,
    pub amount: Money,
    pub account: String,
    pub paid_at: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDebtInput {
    pub customer_name: String,
    pub phone: String,
    pub account_id: String,
    pub amount: Money,
    pub issued_at: String,
    pub due_date: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordPaymentInput {
    pub debt_id: String,
    pub amount: Money,
    pub account_id: String,
    pub paid_at: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelDebtInput {
    pub debt_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub accounts: Vec<Account>,
    pub settings: BusinessSettings,
    pub last_inventory: Inventory,
    pub expected_capital: Money,
    pub last_actual_capital: Money,
    pub open_receivables: Money,
    pub open_debts_count: i64,
    pub overdue_debts_count: i64,
    pub journal_net_since_inventory: Money,
    pub next_inventory_at: String,
    pub inventory_overdue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: serde_json::Value,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportFilters {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportData {
    pub generated_at: String,
    pub filters: ReportFilters,
    pub inventories: Vec<Inventory>,
    pub journal: Vec<JournalEntry>,
    pub debts: Vec<Debt>,
    pub total_positive: Money,
    pub total_negative: Money,
    pub total_variance: Money,
    pub outstanding_receivables: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportInput {
    pub format: String,
    pub destination: String,
    pub filters: ReportFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub app_version: String,
    pub schema_version: i64,
    pub created_at: String,
    pub database_sha256: String,
    pub business_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub created_at: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreInput {
    pub backup_path: String,
    pub recovery_password: String,
    pub new_pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshot {
    pub account_id: String,
    pub provider: String,
    pub name: String,
    pub identifier: Option<String>,
}

impl AccountSnapshot {
    pub fn display_name(&self) -> String {
        let service = match self.provider.as_str() {
            "orange_money" => "Orange Money",
            "wave" => "Wave",
            "djamo" => "Djamo",
            "cash" => "Espèces",
            other => other,
        };
        match &self.identifier {
            Some(identifier) => format!("{} — {} ({})", service, self.name, identifier),
            None => format!("{} — {}", service, self.name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(flatten)]
    pub snapshot: AccountSnapshot,
    pub active: bool,
    pub last_balance: Option<Money>,
    pub last_measured_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountInput {
    pub provider: String,
    pub name: String,
    pub identifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountInput {
    pub account_id: String,
    pub name: String,
    pub identifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpeningAccount {
    pub provider: String,
    pub name: String,
    pub identifier: Option<String>,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalanceInput {
    pub account_id: String,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalanceSnapshot {
    #[serde(flatten)]
    pub account: AccountSnapshot,
    pub amount: Money,
    pub previous_amount: Option<Money>,
    pub delta: Option<Money>,
    pub legacy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpeningPreview {
    pub balances: AccountBalances,
    pub liquidity: Money,
    pub difference: Money,
}
