use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use rusqlite::{backup::Backup, Connection};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    db,
    error::{AppError, AppResult},
    models::{BackupInfo, BackupManifest, RestoreInput},
    security::{self, KeyEnvelope},
    state::{AppPaths, AppState},
};

const DATABASE_ENTRY: &str = "database.db";
const SECURITY_ENTRY: &str = "security.json";
const MANIFEST_ENTRY: &str = "manifest.json";

fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn database_snapshot(
    source: &Connection,
    destination_path: &Path,
    database_key: &[u8],
) -> AppResult<()> {
    if destination_path.exists() {
        fs::remove_file(destination_path)?;
    }
    let mut destination = db::open_database(destination_path, database_key)?;
    {
        let backup = Backup::new(source, &mut destination)?;
        backup.run_to_completion(16, Duration::from_millis(10), None)?;
    }
    db::integrity_check(&destination)?;
    Ok(())
}

fn archive_backup(
    destination: &Path,
    snapshot: &Path,
    security_path: &Path,
    manifest: &BackupManifest,
) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension("tmp");
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    let file = File::create(&temporary)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file(DATABASE_ENTRY, options)?;
    zip.write_all(&fs::read(snapshot)?)?;
    zip.start_file(SECURITY_ENTRY, options)?;
    zip.write_all(&fs::read(security_path)?)?;
    zip.start_file(MANIFEST_ENTRY, options)?;
    zip.write_all(&serde_json::to_vec_pretty(manifest)?)?;
    zip.finish()?;
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary, destination)?;
    Ok(())
}

pub fn before_migration(
    connection: &Connection,
    key: &[u8],
    paths: &AppPaths,
) -> AppResult<PathBuf> {
    let temp = TempDir::new_in(&paths.data_dir)?;
    let snapshot = temp.path().join(DATABASE_ENTRY);
    database_snapshot(connection, &snapshot, key)?;
    let manifest = BackupManifest {
        app_version: env!("CARGO_PKG_VERSION").into(),
        schema_version: db::schema_version(connection)?,
        created_at: Utc::now().to_rfc3339(),
        database_sha256: sha256_file(&snapshot)?,
        business_name: db::get_settings(connection)?.business_name,
    };
    let path = paths.backups.join(format!(
        "avant-migration-v{}-{}.msbackup",
        manifest.schema_version,
        uuid::Uuid::new_v4()
    ));
    archive_backup(&path, &snapshot, &paths.security, &manifest)?;
    // Verify the completed archive before touching the source database.
    extract_and_validate(&path)?;
    Ok(path)
}

pub fn create_backup(state: &AppState, destination: Option<PathBuf>) -> AppResult<BackupInfo> {
    let key = state.database_key()?;
    let timestamp = Utc::now();
    let default_name = format!("ker-finance-{}.msbackup", timestamp.format("%Y%m%d-%H%M%S"));
    let destination = destination.unwrap_or_else(|| state.paths.backups.join(default_name));
    if destination == state.paths.database || destination == state.paths.security {
        return Err(AppError::Validation(
            "Le fichier de sauvegarde ne peut pas remplacer les données actives.".into(),
        ));
    }

    let temp = TempDir::new_in(&state.paths.data_dir)?;
    let snapshot = temp.path().join(DATABASE_ENTRY);
    state.with_connection(|connection| {
        db::integrity_check(connection)?;
        database_snapshot(connection, &snapshot, &key)
    })?;
    let settings = state.with_connection(|connection| db::get_settings(connection))?;
    let manifest = BackupManifest {
        app_version: env!("CARGO_PKG_VERSION").into(),
        schema_version: state.with_connection(|connection| db::schema_version(connection))?,
        created_at: timestamp.to_rfc3339(),
        database_sha256: sha256_file(&snapshot)?,
        business_name: settings.business_name,
    };
    archive_backup(&destination, &snapshot, &state.paths.security, &manifest)?;
    let size_bytes = fs::metadata(&destination)?.len();
    Ok(BackupInfo {
        path: destination.to_string_lossy().to_string(),
        created_at: manifest.created_at,
        size_bytes,
    })
}

pub fn prune_local_backups(state: &AppState) -> AppResult<()> {
    let mut backups = list_backups(state)?;
    if backups.len() <= 30 {
        return Ok(());
    }
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    for backup in backups.into_iter().skip(30) {
        let path = PathBuf::from(backup.path);
        if path.parent() == Some(state.paths.backups.as_path()) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub fn list_backups(state: &AppState) -> AppResult<Vec<BackupInfo>> {
    let mut values = Vec::new();
    if !state.paths.backups.exists() {
        return Ok(values);
    }
    for entry in fs::read_dir(&state.paths.backups)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("msbackup") {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified = metadata
            .modified()
            .ok()
            .map(chrono::DateTime::<Utc>::from)
            .unwrap_or_else(Utc::now);
        values.push(BackupInfo {
            path: path.to_string_lossy().to_string(),
            created_at: modified.to_rfc3339(),
            size_bytes: metadata.len(),
        });
    }
    values.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(values)
}

struct ExtractedBackup {
    _temp: TempDir,
    database: PathBuf,
    envelope: KeyEnvelope,
    manifest: BackupManifest,
}

fn extract_and_validate(path: &Path) -> AppResult<ExtractedBackup> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let temp = TempDir::new()?;
    let database = temp.path().join(DATABASE_ENTRY);

    let mut db_entry = archive.by_name(DATABASE_ENTRY)?;
    let mut db_file = File::create(&database)?;
    std::io::copy(&mut db_entry, &mut db_file)?;
    drop(db_entry);

    let mut security_bytes = Vec::new();
    archive
        .by_name(SECURITY_ENTRY)?
        .read_to_end(&mut security_bytes)?;
    let envelope: KeyEnvelope = serde_json::from_slice(&security_bytes)?;

    let mut manifest_bytes = Vec::new();
    archive
        .by_name(MANIFEST_ENTRY)?
        .read_to_end(&mut manifest_bytes)?;
    let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema_version > db::SCHEMA_VERSION {
        return Err(AppError::Validation(
            "Cette sauvegarde provient d’une version plus récente de l’application.".into(),
        ));
    }
    if sha256_file(&database)? != manifest.database_sha256 {
        return Err(AppError::Validation(
            "La sauvegarde est endommagée ou incomplète.".into(),
        ));
    }
    Ok(ExtractedBackup {
        _temp: temp,
        database,
        envelope,
        manifest,
    })
}

pub fn restore_backup(state: &AppState, input: RestoreInput) -> AppResult<BackupInfo> {
    restore_backup_with(state, input, security::rewrap_recovered_key)
}

fn restore_backup_with(
    state: &AppState,
    input: RestoreInput,
    rewrap: impl FnOnce(&[u8], &str, &str) -> AppResult<KeyEnvelope>,
) -> AppResult<BackupInfo> {
    crate::domain::validate_pin(&input.new_pin)?;
    let backup_path = PathBuf::from(&input.backup_path);
    if !backup_path.is_file() {
        return Err(AppError::Validation(
            "Le fichier de sauvegarde est introuvable.".into(),
        ));
    }
    let extracted = extract_and_validate(&backup_path)?;
    let database_key =
        security::unlock_with_recovery(&extracted.envelope, &input.recovery_password)?;
    let restored_connection = db::open_database(&extracted.database, &database_key)?;
    db::integrity_check(&restored_connection)?;
    db::get_settings(&restored_connection)?;
    if db::schema_version(&restored_connection)? != extracted.manifest.schema_version {
        return Err(AppError::Validation(
            "Version de sauvegarde incohérente.".into(),
        ));
    }
    db::migrate(&restored_connection)?;
    db::integrity_check(&restored_connection)?;
    drop(restored_connection);

    if state.setup_status().initialized && state.setup_status().unlocked {
        let _ = create_backup(state, None)?;
    }
    state.lock();
    let rollback_database = state.paths.data_dir.join("database.before-restore");
    let rollback_security = state.paths.data_dir.join("security.before-restore");
    let _ = fs::remove_file(&rollback_database);
    let _ = fs::remove_file(&rollback_security);
    if state.paths.database.exists() {
        fs::rename(&state.paths.database, &rollback_database)?;
    }
    if state.paths.security.exists() {
        fs::rename(&state.paths.security, &rollback_security)?;
    }

    let restore_result = (|| {
        fs::copy(&extracted.database, &state.paths.database)?;
        let envelope = rewrap(&database_key, &input.new_pin, &input.recovery_password)?;
        security::write_envelope(&state.paths.security, &envelope)?;
        state.set_recovered_session(database_key)?;
        Ok(())
    })();

    if let Err(error) = restore_result {
        let _ = fs::remove_file(&state.paths.database);
        let _ = fs::remove_file(&state.paths.security);
        if rollback_database.exists() {
            let _ = fs::rename(&rollback_database, &state.paths.database);
        }
        if rollback_security.exists() {
            let _ = fs::rename(&rollback_security, &state.paths.security);
        }
        return Err(error);
    }
    let _ = fs::remove_file(rollback_database);
    let _ = fs::remove_file(rollback_security);
    Ok(BackupInfo {
        path: backup_path.to_string_lossy().to_string(),
        created_at: extracted.manifest.created_at,
        size_bytes: fs::metadata(backup_path)?.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{accounts, db::test_support::*, models::*};

    fn snapshot(connection: &Connection) -> serde_json::Value {
        serde_json::json!({
            "accounts": accounts::list(connection).unwrap(),
            "inventories": db::list_inventories(connection,None).unwrap(),
            "journal": db::list_journal_entries(connection,None).unwrap(),
            "debts": db::list_debts(connection,None).unwrap(),
            "audit": db::list_audit_events(connection,500).unwrap(),
        })
    }

    #[test]
    fn migration_requires_a_verified_encrypted_backup_before_any_change() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path().to_path_buf()).unwrap();
        let key = vec![17u8; 32];
        let connection = db::open_database(&state.paths.database, &key).unwrap();
        legacy_database(&connection);
        let before = historical_data(&connection);
        // Missing envelope makes the safety backup fail: database must remain v1.
        assert!(state.set_recovered_session(key.clone().into()).is_err());
        assert_eq!(db::schema_version(&connection).unwrap(), 1);
        assert_eq!(historical_data(&connection), before);
        assert!(!state.setup_status().unlocked);
        let envelope =
            security::test_envelope(&key, "123456", "une phrase de récupération solide").unwrap();
        security::write_envelope(&state.paths.security, &envelope).unwrap();
        state.set_recovered_session(key.clone().into()).unwrap();
        assert_eq!(
            db::schema_version(&connection).unwrap(),
            crate::db::SCHEMA_VERSION
        );
        let backups = list_backups(&state).unwrap();
        assert_eq!(backups.len(), 1);
        let extracted = extract_and_validate(Path::new(&backups[0].path)).unwrap();
        assert_eq!(extracted.manifest.schema_version, 1);
        assert!(!fs::read(&extracted.database)
            .unwrap()
            .starts_with(b"SQLite format 3"));
        let original = db::open_database(&extracted.database, &key).unwrap();
        assert_eq!(db::schema_version(&original).unwrap(), 1);
        assert_eq!(historical_data(&original), before);
        state.with_connection(|_| Ok(())).unwrap();
        assert_eq!(list_backups(&state).unwrap().len(), 1);
    }

    #[test]
    fn restores_legacy_and_current_archives_and_preserves_multi_account_history() {
        const PASSWORD: &str = "une phrase de récupération solide";
        let source_dir = TempDir::new().unwrap();
        let source = AppState::new(source_dir.path().to_path_buf()).unwrap();
        let key = vec![31u8; 32];
        let connection = db::open_database(&source.paths.database, &key).unwrap();
        legacy_database(&connection);
        let legacy = historical_data(&connection);
        drop(connection);
        security::write_envelope(
            &source.paths.security,
            &security::test_envelope(&key, "123456", PASSWORD).unwrap(),
        )
        .unwrap();
        source.set_recovered_session(key.clone().into()).unwrap();
        let v1 = list_backups(&source).unwrap().remove(0);
        source
            .with_connection(|db| {
                let extra = accounts::create(
                    db,
                    CreateAccountInput {
                        provider: "wave".into(),
                        name: "Wave 2".into(),
                        identifier: Some("771234567".into()),
                    },
                )?;
                let loan = debt(db, "legacy-djamo");
                payment(db, &loan.id, &extra.snapshot.account_id, 10_000);
                accounts::update(
                    db,
                    UpdateAccountInput {
                        account_id: "legacy-djamo".into(),
                        name: "Djamo renommé".into(),
                        identifier: Some("DJ2".into()),
                    },
                )?;
                let unused = accounts::create(
                    db,
                    CreateAccountInput {
                        provider: "orange_money".into(),
                        name: "Réserve".into(),
                        identifier: None,
                    },
                )?;
                accounts::set_active(db, &unused.snapshot.account_id, false)?;
                let values = balances(db);
                close(db, values);
                Ok(())
            })
            .unwrap();
        let current = create_backup(&source, None).unwrap();
        let expected = source.with_connection(|db| Ok(snapshot(db))).unwrap();
        for (version, backup) in [(1, v1), (db::SCHEMA_VERSION, current)] {
            let target_dir = TempDir::new().unwrap();
            let target = AppState::new(target_dir.path().to_path_buf()).unwrap();
            let input = RestoreInput {
                backup_path: backup.path,
                recovery_password: PASSWORD.into(),
                new_pin: "654321".into(),
            };
            if version == db::SCHEMA_VERSION {
                // Exercise replacement of an existing unlocked installation as well.
                let mut existing = db::open_database(&target.paths.database, &[22u8; 32]).unwrap();
                db::migrate(&existing).unwrap();
                db::initialize_business(&mut existing, &multi_setup()).unwrap();
                security::write_envelope(
                    &target.paths.security,
                    &security::test_envelope(&[22u8; 32], "123456", PASSWORD).unwrap(),
                )
                .unwrap();
                target.set_recovered_session(vec![22u8; 32].into()).unwrap();
            }
            let before_attempt = if target.setup_status().unlocked {
                Some(target.with_connection(|db| Ok(snapshot(db))).unwrap())
            } else {
                None
            };
            let wrong = RestoreInput {
                recovery_password: "un mot de passe erroné".into(),
                ..input.clone()
            };
            assert!(restore_backup_with(&target, wrong, security::test_envelope).is_err());
            if let Some(before) = before_attempt {
                assert_eq!(
                    target.with_connection(|db| Ok(snapshot(db))).unwrap(),
                    before
                );
            } else {
                assert!(!target.paths.database.exists());
            }
            restore_backup_with(&target, input, security::test_envelope).unwrap();
            assert!(target.setup_status().unlocked);
            target
                .with_connection(|db| {
                    assert_eq!(db::schema_version(db)?, crate::db::SCHEMA_VERSION);
                    if version == 1 {
                        assert_eq!(historical_data(db), legacy);
                        assert_eq!(accounts::list(db)?.len(), 4);
                        assert_eq!(
                            db::list_audit_events(db, 500)?
                                .iter()
                                .filter(|a| a.action == "schema_migrated")
                                .count(),
                            1
                        );
                    } else {
                        assert_eq!(snapshot(db), expected);
                    }
                    db::integrity_check(db)
                })
                .unwrap();
        }
    }

    #[test]
    fn restores_v2_then_roundtrips_stock_and_idempotency_in_encrypted_v3_backup() {
        use crate::stock;
        const PASSWORD: &str = "une phrase de récupération solide";
        let source_dir = TempDir::new().unwrap();
        let source = AppState::new(source_dir.path().to_path_buf()).unwrap();
        let key = vec![41u8; 32];
        let connection = db::open_database(&source.paths.database, &key).unwrap();
        legacy_database(&connection);
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection
            .execute_batch(include_str!("migration_v2.sql"))
            .unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        let historical = historical_data(&connection);
        security::write_envelope(
            &source.paths.security,
            &security::test_envelope(&key, "123456", PASSWORD).unwrap(),
        )
        .unwrap();
        let v2 = before_migration(&connection, &key, &source.paths).unwrap();
        assert_eq!(
            extract_and_validate(&v2).unwrap().manifest.schema_version,
            2
        );
        drop(connection);
        let restored_dir = TempDir::new().unwrap();
        let restored = AppState::new(restored_dir.path().to_path_buf()).unwrap();
        restore_backup_with(
            &restored,
            RestoreInput {
                backup_path: v2.to_string_lossy().into(),
                recovery_password: PASSWORD.into(),
                new_pin: "654321".into(),
            },
            security::test_envelope,
        )
        .unwrap();
        let retry = restored
            .with_connection(|db| {
                assert_eq!(historical_data(db), historical);
                assert!(stock::products(db)?.is_empty());
                let product = stock::create_product(
                    db,
                    CreateProductInput {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        name: "Chargeur".into(),
                        price: 2000,
                        initial_stock: 10,
                    },
                )?;
                let sale = CreateSaleInput {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    account_id: "legacy-cash".into(),
                    occurred_at: "2026-09-07".into(),
                    note: None,
                    lines: vec![SaleLineInput {
                        product_id: product.id.clone(),
                        quantity: 2,
                        unit_price: 1750,
                    }],
                };
                stock::create_sale(db, sale.clone())?;
                let receipt = stock::receive_stock(
                    db,
                    ReceiveStockInput {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        product_id: product.id.clone(),
                        quantity: 5,
                        amount: 6000,
                        account_id: "legacy-wave".into(),
                        occurred_at: "2026-09-07".into(),
                        note: Some("Livraison".into()),
                    },
                )?;
                stock::cancel_operation(
                    db,
                    CancelProductOperationInput {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        operation_id: receipt.id,
                        reason: "Achat saisi en double".into(),
                    },
                )?;
                stock::adjust_stock(
                    db,
                    AdjustStockInput {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        product_id: product.id,
                        quantity: 7,
                        reason: "Un article abîmé".into(),
                    },
                )?;
                Ok(sale)
            })
            .unwrap();
        let stock_snapshot = |db: &Connection| -> AppResult<serde_json::Value> {
            Ok(
                serde_json::json!({ "products": stock::products(db)?, "operations": stock::operations(db)?,
                "movements": stock::movements(db, None)?, "finance": snapshot(db) }),
            )
        };
        let expected = restored.with_connection(|db| stock_snapshot(db)).unwrap();
        let archive = create_backup(&restored, None).unwrap();
        assert_eq!(
            extract_and_validate(Path::new(&archive.path))
                .unwrap()
                .manifest
                .schema_version,
            3
        );
        let target_dir = TempDir::new().unwrap();
        let target = AppState::new(target_dir.path().to_path_buf()).unwrap();
        restore_backup_with(
            &target,
            RestoreInput {
                backup_path: archive.path,
                recovery_password: PASSWORD.into(),
                new_pin: "123456".into(),
            },
            security::test_envelope,
        )
        .unwrap();
        target
            .with_connection(|db| {
                assert_eq!(stock_snapshot(db)?, expected);
                stock::create_sale(db, retry)?;
                assert_eq!(stock_snapshot(db)?, expected);
                db::integrity_check(db)
            })
            .unwrap();
    }

    #[test]
    fn backup_archive_roundtrip_checks_hash_and_manifest() {
        let temp = TempDir::new().unwrap();
        let database = temp.path().join("source.db");
        let security = temp.path().join("security.json");
        let archive = temp.path().join("test.msbackup");
        fs::write(&database, b"encrypted database content").unwrap();
        let envelope = KeyEnvelope {
            version: 1,
            local_salt: "a".into(),
            local_nonce: "b".into(),
            local_ciphertext: "c".into(),
            recovery_salt: "d".into(),
            recovery_nonce: "e".into(),
            recovery_ciphertext: "f".into(),
        };
        fs::write(&security, serde_json::to_vec(&envelope).unwrap()).unwrap();
        let manifest = BackupManifest {
            app_version: "0.1.0".into(),
            schema_version: db::SCHEMA_VERSION,
            created_at: "2026-08-26T12:00:00Z".into(),
            database_sha256: sha256_file(&database).unwrap(),
            business_name: "Boutique".into(),
        };
        archive_backup(&archive, &database, &security, &manifest).unwrap();
        let extracted = extract_and_validate(&archive).unwrap();
        assert_eq!(extracted.manifest.business_name, "Boutique");
        assert_eq!(
            fs::read(extracted.database).unwrap(),
            b"encrypted database content"
        );
    }
}
