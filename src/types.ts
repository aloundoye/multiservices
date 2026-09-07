export type Money = number;

export interface SetupStatus {
  initialized: boolean;
  unlocked: boolean;
}

export interface SetupInput {
  businessName: string;
  pin: string;
  recoveryPassword: string;
  initialCapital: Money;
  accounts: OpeningAccount[];
}

export interface BusinessSettings {
  businessName: string;
  currency: string;
  timezone: string;
  inventoryIntervalMinutes: number;
  autoLockMinutes: number;
  createdAt: string;
}

export interface AccountBalances {
  orangeMoney: Money;
  wave: Money;
  djamo: Money;
  cash: Money;
}

export interface Inventory {
  accountBalances: AccountBalanceSnapshot[];
  id: string;
  kind: "opening" | "regular";
  closedAt: string;
  balances: AccountBalances;
  receivables: Money;
  liquidity: Money;
  expectedTotal: Money;
  actualTotal: Money;
  variance: Money;
  varianceCategory?: string;
  varianceNote?: string;
  delta: AccountBalances;
}

export interface InventoryPreview {
  accountBalances: AccountBalanceSnapshot[];
  balances: AccountBalances;
  previousBalances: AccountBalances;
  delta: AccountBalances;
  receivables: Money;
  liquidity: Money;
  expectedTotal: Money;
  actualTotal: Money;
  variance: Money;
}

export interface CloseInventoryResult {
  inventory: Inventory;
  backup?: BackupInfo;
  backupWarning?: string;
}

export interface JournalEntry {
  accountSnapshot: AccountSnapshot;
  id: string;
  entryType: string;
  amount: Money;
  signedAmount: Money;
  paymentAccount: string;
  occurredAt: string;
  postedAt: string;
  reference?: string;
  note?: string;
  reversesId?: string;
  reversed: boolean;
}

export interface DebtPayment {
  accountSnapshot: AccountSnapshot;
  id: string;
  debtId: string;
  amount: Money;
  account: string;
  paidAt: string;
  note?: string;
  createdAt: string;
}

export interface Debt {
  accountSnapshot: AccountSnapshot;
  id: string;
  customerName: string;
  phone: string;
  provider: MobileProvider;
  principal: Money;
  remaining: Money;
  issuedAt: string;
  dueDate?: string;
  note?: string;
  status: "open" | "partial" | "paid" | "overdue" | "cancelled";
  createdAt: string;
  payments: DebtPayment[];
}

export interface Dashboard {
  accounts: Account[];
  settings: BusinessSettings;
  lastInventory: Inventory;
  expectedCapital: Money;
  lastActualCapital: Money;
  openReceivables: Money;
  openDebtsCount: number;
  overdueDebtsCount: number;
  journalNetSinceInventory: Money;
  nextInventoryAt: string;
  inventoryOverdue: boolean;
}

export interface ReportFilters {
  from?: string;
  to?: string;
}

export interface ReportData {
  generatedAt: string;
  filters: ReportFilters;
  inventories: Inventory[];
  journal: JournalEntry[];
  debts: Debt[];
  totalPositive: Money;
  totalNegative: Money;
  totalVariance: Money;
  outstandingReceivables: Money;
}

export interface AuditEvent {
  id: string;
  action: string;
  entityType: string;
  entityId?: string;
  details: Record<string, unknown>;
  occurredAt: string;
}

export interface BackupInfo {
  path: string;
  createdAt: string;
  sizeBytes: number;
}

export type PageId =
  | "dashboard"
  | "inventory"
  | "history"
  | "journal"
  | "debts"
  | "reports"
  | "settings";

export type MobileProvider = "orange_money" | "wave" | "djamo";
export type Provider = MobileProvider | "cash";
export interface AccountSnapshot {
  accountId: string;
  provider: Provider;
  name: string;
  identifier?: string | null;
}
export interface Account extends AccountSnapshot {
  active: boolean;
  lastBalance?: Money | null;
  lastMeasuredAt?: string | null;
}
export interface AccountBalanceSnapshot extends AccountSnapshot {
  amount: Money;
  previousAmount?: Money | null;
  delta?: Money | null;
  legacy: boolean;
}
export interface AccountBalanceInput { accountId: string; amount: Money }
export interface OpeningAccount {
  provider: Provider;
  name: string;
  identifier?: string | null;
  amount: Money;
}
export interface CreateAccountInput { provider: MobileProvider; name: string; identifier?: string | null }
export interface UpdateAccountInput { accountId: string; name: string; identifier?: string | null }
export interface OpeningPreview { balances: AccountBalances; liquidity: Money; difference: Money }
export interface CloseInventoryInput { balances: AccountBalanceInput[]; varianceCategory?: string | null; varianceNote?: string | null }
export interface InventoryCorrectionInput { inventoryId: string; amount: Money; direction: string; accountId: string; reason: string }
export interface CreateJournalEntryInput { entryType: string; amount: Money; accountId: string; occurredAt: string; reference?: string | null; note?: string | null }
export interface CreateDebtInput { customerName: string; phone: string; accountId: string; amount: Money; issuedAt: string; dueDate?: string | null; note?: string | null }
export interface RecordPaymentInput { debtId: string; amount: Money; accountId: string; paidAt: string; note?: string | null }
