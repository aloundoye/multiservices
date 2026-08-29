use std::path::PathBuf;

use tauri::State;

use crate::{backup, db, export, models::*, state::AppState};

type CommandResult<T> = Result<T, String>;

fn command_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
pub fn check_setup(state: State<'_, AppState>) -> SetupStatus {
    state.setup_status()
}

#[tauri::command]
pub fn setup_business(input: SetupInput, state: State<'_, AppState>) -> CommandResult<Dashboard> {
    state.setup(input).map_err(command_error)?;
    let dashboard = state
        .with_connection(|connection| db::get_dashboard(connection))
        .map_err(command_error)?;
    let _ = backup::create_backup(&state, None).and_then(|_| backup::prune_local_backups(&state));
    Ok(dashboard)
}

#[tauri::command]
pub fn login(input: LoginInput, state: State<'_, AppState>) -> CommandResult<Dashboard> {
    state.login(input).map_err(command_error)?;
    state
        .with_connection(|connection| db::get_dashboard(connection))
        .map_err(command_error)
}

#[tauri::command]
pub fn lock_app(state: State<'_, AppState>) {
    state.lock();
}

#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> CommandResult<Dashboard> {
    state
        .with_connection(|connection| db::get_dashboard(connection))
        .map_err(command_error)
}

#[tauri::command]
pub fn preview_inventory(
    input: InventoryPreviewInput,
    state: State<'_, AppState>,
) -> CommandResult<InventoryPreview> {
    state
        .with_connection(|connection| db::preview_inventory(connection, input))
        .map_err(command_error)
}

#[tauri::command]
pub fn close_inventory(
    input: CloseInventoryInput,
    state: State<'_, AppState>,
) -> CommandResult<CloseInventoryResult> {
    let inventory = state
        .with_connection(|connection| db::close_inventory(connection, input))
        .map_err(command_error)?;
    let (backup, backup_warning) = match backup::create_backup(&state, None) {
        Ok(info) => {
            let warning = backup::prune_local_backups(&state)
                .err()
                .map(|error| error.to_string());
            (Some(info), warning)
        }
        Err(error) => (None, Some(error.to_string())),
    };
    Ok(CloseInventoryResult {
        inventory,
        backup,
        backup_warning,
    })
}

#[tauri::command]
pub fn create_inventory_correction(
    input: InventoryCorrectionInput,
    state: State<'_, AppState>,
) -> CommandResult<JournalEntry> {
    state
        .with_connection(|connection| db::create_inventory_correction(connection, input))
        .map_err(command_error)
}

#[tauri::command]
pub fn list_inventories(state: State<'_, AppState>) -> CommandResult<Vec<Inventory>> {
    state
        .with_connection(|connection| db::list_inventories(connection, None))
        .map_err(command_error)
}

#[tauri::command]
pub fn list_journal_entries(state: State<'_, AppState>) -> CommandResult<Vec<JournalEntry>> {
    state
        .with_connection(|connection| db::list_journal_entries(connection, None))
        .map_err(command_error)
}

#[tauri::command]
pub fn create_journal_entry(
    input: CreateJournalEntryInput,
    state: State<'_, AppState>,
) -> CommandResult<JournalEntry> {
    state
        .with_connection(|connection| db::create_journal_entry(connection, input))
        .map_err(command_error)
}

#[tauri::command]
pub fn reverse_journal_entry(
    input: ReverseEntryInput,
    state: State<'_, AppState>,
) -> CommandResult<JournalEntry> {
    state
        .with_connection(|connection| db::reverse_journal_entry(connection, input))
        .map_err(command_error)
}

#[tauri::command]
pub fn list_debts(state: State<'_, AppState>) -> CommandResult<Vec<Debt>> {
    state
        .with_connection(|connection| db::list_debts(connection, None))
        .map_err(command_error)
}

#[tauri::command]
pub fn create_debt(input: CreateDebtInput, state: State<'_, AppState>) -> CommandResult<Debt> {
    state
        .with_connection(|connection| db::create_debt(connection, input))
        .map_err(command_error)
}

#[tauri::command]
pub fn record_debt_payment(
    input: RecordPaymentInput,
    state: State<'_, AppState>,
) -> CommandResult<Debt> {
    state
        .with_connection(|connection| db::record_debt_payment(connection, input))
        .map_err(command_error)
}

#[tauri::command]
pub fn cancel_debt(input: CancelDebtInput, state: State<'_, AppState>) -> CommandResult<Debt> {
    state
        .with_connection(|connection| db::cancel_debt(connection, input))
        .map_err(command_error)
}

#[tauri::command]
pub fn get_report(filters: ReportFilters, state: State<'_, AppState>) -> CommandResult<ReportData> {
    state
        .with_connection(|connection| db::get_report(connection, filters))
        .map_err(command_error)
}

#[tauri::command]
pub fn export_report(input: ExportInput, state: State<'_, AppState>) -> CommandResult<String> {
    let report = state
        .with_connection(|connection| db::get_report(connection, input.filters.clone()))
        .map_err(command_error)?;
    export::export_report(&input, &report).map_err(command_error)
}

#[tauri::command]
pub fn list_audit_events(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> CommandResult<Vec<AuditEvent>> {
    state
        .with_connection(|connection| db::list_audit_events(connection, limit.unwrap_or(100)))
        .map_err(command_error)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CommandResult<BusinessSettings> {
    state
        .with_connection(|connection| db::get_settings(connection))
        .map_err(command_error)
}

#[tauri::command]
pub fn update_settings(
    input: UpdateSettingsInput,
    state: State<'_, AppState>,
) -> CommandResult<BusinessSettings> {
    let settings = state
        .with_connection(|connection| db::update_settings(connection, input))
        .map_err(command_error)?;
    state.update_session_timeout(settings.auto_lock_minutes);
    Ok(settings)
}

#[tauri::command]
pub fn create_backup(
    destination: Option<String>,
    state: State<'_, AppState>,
) -> CommandResult<BackupInfo> {
    let destination = destination.map(PathBuf::from);
    let info = backup::create_backup(&state, destination).map_err(command_error)?;
    backup::prune_local_backups(&state).map_err(command_error)?;
    Ok(info)
}

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>) -> CommandResult<Vec<BackupInfo>> {
    backup::list_backups(&state).map_err(command_error)
}

#[tauri::command]
pub fn restore_backup(
    input: RestoreInput,
    state: State<'_, AppState>,
) -> CommandResult<BackupInfo> {
    backup::restore_backup(&state, input).map_err(command_error)
}
