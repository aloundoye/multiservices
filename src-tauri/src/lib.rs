mod backup;
mod commands;
mod db;
mod domain;
mod error;
mod export;
mod models;
mod security;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            let state = state::AppState::new(data_dir).map_err(|error| error.to_string())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_setup,
            commands::setup_business,
            commands::login,
            commands::lock_app,
            commands::get_dashboard,
            commands::preview_inventory,
            commands::close_inventory,
            commands::create_inventory_correction,
            commands::list_inventories,
            commands::list_journal_entries,
            commands::create_journal_entry,
            commands::reverse_journal_entry,
            commands::list_debts,
            commands::create_debt,
            commands::record_debt_payment,
            commands::cancel_debt,
            commands::get_report,
            commands::export_report,
            commands::list_audit_events,
            commands::get_settings,
            commands::update_settings,
            commands::create_backup,
            commands::list_backups,
            commands::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("erreur pendant l’exécution de Kër Finance");
}
