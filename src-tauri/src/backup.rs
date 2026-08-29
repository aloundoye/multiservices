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
    state::AppState,
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
        schema_version: db::SCHEMA_VERSION,
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
        let envelope = security::rewrap_recovered_key(
            &database_key,
            &input.new_pin,
            &input.recovery_password,
        )?;
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
