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
  orangeMoney: Money;
  wave: Money;
  djamo: Money;
  cash: Money;
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
  id: string;
  debtId: string;
  amount: Money;
  account: string;
  paidAt: string;
  note?: string;
  createdAt: string;
}

export interface Debt {
  id: string;
  customerName: string;
  phone: string;
  provider: "orange_money" | "wave";
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
