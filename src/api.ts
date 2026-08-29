import { invoke } from "@tauri-apps/api/core";
import type {
  AuditEvent,
  BackupInfo,
  BusinessSettings,
  CloseInventoryResult,
  Dashboard,
  Debt,
  Inventory,
  InventoryPreview,
  JournalEntry,
  ReportData,
  ReportFilters,
  SetupInput,
  SetupStatus
} from "./types";

export class AppApiError extends Error {
  isLocked: boolean;

  constructor(message: string) {
    super(message);
    this.name = "AppApiError";
    this.isLocked = message.toLowerCase().includes("session verrouillée");
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const message = typeof error === "string" ? error : error instanceof Error ? error.message : "Erreur inconnue";
    throw new AppApiError(message);
  }
}

export const api = {
  checkSetup: () => {
    const tauriAvailable = Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
    return !tauriAvailable && import.meta.env.DEV
      ? Promise.resolve<SetupStatus>({ initialized: false, unlocked: false })
      : call<SetupStatus>("check_setup");
  },
  setup: (input: SetupInput) => call<Dashboard>("setup_business", { input }),
  login: (pin: string) => call<Dashboard>("login", { input: { pin } }),
  lock: () => call<void>("lock_app"),
  dashboard: () => call<Dashboard>("get_dashboard"),
  previewInventory: (input: Record<string, number>) =>
    call<InventoryPreview>("preview_inventory", { input }),
  closeInventory: (input: Record<string, unknown>) =>
    call<CloseInventoryResult>("close_inventory", { input }),
  correctInventory: (input: Record<string, unknown>) =>
    call<JournalEntry>("create_inventory_correction", { input }),
  inventories: () => call<Inventory[]>("list_inventories"),
  journal: () => call<JournalEntry[]>("list_journal_entries"),
  createJournal: (input: Record<string, unknown>) =>
    call<JournalEntry>("create_journal_entry", { input }),
  reverseJournal: (entryId: string, reason: string) =>
    call<JournalEntry>("reverse_journal_entry", { input: { entryId, reason } }),
  debts: () => call<Debt[]>("list_debts"),
  createDebt: (input: Record<string, unknown>) => call<Debt>("create_debt", { input }),
  payDebt: (input: Record<string, unknown>) => call<Debt>("record_debt_payment", { input }),
  cancelDebt: (debtId: string, reason: string) =>
    call<Debt>("cancel_debt", { input: { debtId, reason } }),
  report: (filters: ReportFilters) => call<ReportData>("get_report", { filters }),
  exportReport: (format: string, destination: string, filters: ReportFilters) =>
    call<string>("export_report", { input: { format, destination, filters } }),
  audit: (limit = 100) => call<AuditEvent[]>("list_audit_events", { limit }),
  settings: () => call<BusinessSettings>("get_settings"),
  updateSettings: (input: Record<string, unknown>) =>
    call<BusinessSettings>("update_settings", { input }),
  backups: () => call<BackupInfo[]>("list_backups"),
  createBackup: (destination?: string) => call<BackupInfo>("create_backup", { destination }),
  restoreBackup: (backupPath: string, recoveryPassword: string, newPin: string) =>
    call<BackupInfo>("restore_backup", { input: { backupPath, recoveryPassword, newPin } })
};
