use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use rusqlite::Connection;
use zeroize::Zeroizing;

use crate::{
    accounts, backup, db,
    error::{AppError, AppResult},
    models::{LoginInput, SetupInput, SetupStatus},
    security,
};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub database: PathBuf,
    pub security: PathBuf,
    pub backups: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            database: data_dir.join("ker-finance.db"),
            security: data_dir.join("security.json"),
            backups: data_dir.join("backups"),
            data_dir,
        }
    }
}

struct UnlockedSession {
    database_key: Zeroizing<Vec<u8>>,
    last_activity: Instant,
    auto_lock_minutes: i64,
}

pub struct AppState {
    pub paths: AppPaths,
    session: Mutex<Option<UnlockedSession>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> AppResult<Self> {
        fs::create_dir_all(&data_dir)?;
        let paths = AppPaths::new(data_dir);
        fs::create_dir_all(&paths.backups)?;
        Ok(Self {
            paths,
            session: Mutex::new(None),
        })
    }

    fn migrate_safely(&self, connection: &Connection, key: &[u8]) -> AppResult<()> {
        if db::schema_version(connection)? == 1 {
            backup::before_migration(connection, key, &self.paths)?;
        }
        db::migrate(connection)
    }

    pub fn setup_status(&self) -> SetupStatus {
        SetupStatus {
            initialized: self.paths.database.exists() && self.paths.security.exists(),
            unlocked: self.session.lock().map(|s| s.is_some()).unwrap_or(false),
        }
    }

    pub fn setup(&self, input: SetupInput) -> AppResult<()> {
        if self.paths.database.exists() || self.paths.security.exists() {
            return Err(AppError::Validation(
                "L’application a déjà été initialisée.".into(),
            ));
        }
        accounts::validate_opening(&input)?;
        let (envelope, database_key) =
            security::create_key_envelope(&input.pin, &input.recovery_password)?;
        security::write_envelope(&self.paths.security, &envelope)?;

        let result = (|| {
            let mut connection = db::open_database(&self.paths.database, &database_key)?;
            self.migrate_safely(&connection, &database_key)?;
            db::initialize_business(&mut connection, &input)?;
            db::integrity_check(&connection)?;
            let settings = db::get_settings(&connection)?;
            *self
                .session
                .lock()
                .map_err(|_| AppError::Internal("Session indisponible.".into()))? =
                Some(UnlockedSession {
                    database_key,
                    last_activity: Instant::now(),
                    auto_lock_minutes: settings.auto_lock_minutes,
                });
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&self.paths.database);
            let _ = fs::remove_file(&self.paths.security);
        }
        result
    }

    pub fn login(&self, input: LoginInput) -> AppResult<()> {
        if !self.paths.database.exists() || !self.paths.security.exists() {
            return Err(AppError::Validation(
                "L’application n’a pas encore été initialisée.".into(),
            ));
        }
        let envelope = security::read_envelope(&self.paths.security)?;
        let database_key = security::unlock_with_pin(&envelope, &input.pin)?;
        let connection = db::open_database(&self.paths.database, &database_key)?;
        self.migrate_safely(&connection, &database_key)?;
        let settings = db::get_settings(&connection)?;
        *self
            .session
            .lock()
            .map_err(|_| AppError::Internal("Session indisponible.".into()))? =
            Some(UnlockedSession {
                database_key,
                last_activity: Instant::now(),
                auto_lock_minutes: settings.auto_lock_minutes,
            });
        Ok(())
    }

    pub fn lock(&self) {
        if let Ok(mut session) = self.session.lock() {
            *session = None;
        }
    }

    pub fn update_session_timeout(&self, minutes: i64) {
        if let Ok(mut guard) = self.session.lock() {
            if let Some(session) = guard.as_mut() {
                session.auto_lock_minutes = minutes;
                session.last_activity = Instant::now();
            }
        }
    }

    fn active_key(&self) -> AppResult<Zeroizing<Vec<u8>>> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| AppError::Internal("Session indisponible.".into()))?;
        let session = guard.as_mut().ok_or(AppError::Locked)?;
        if session.last_activity.elapsed()
            > Duration::from_secs((session.auto_lock_minutes.max(1) * 60) as u64)
        {
            *guard = None;
            return Err(AppError::Locked);
        }
        session.last_activity = Instant::now();
        Ok(session.database_key.clone())
    }

    pub fn database_key(&self) -> AppResult<Zeroizing<Vec<u8>>> {
        self.active_key()
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> AppResult<T>,
    ) -> AppResult<T> {
        let key = self.active_key()?;
        let mut connection = db::open_database(&self.paths.database, &key)?;
        self.migrate_safely(&connection, &key)?;
        operation(&mut connection)
    }

    pub fn set_recovered_session(&self, key: Zeroizing<Vec<u8>>) -> AppResult<()> {
        let connection = db::open_database(&self.paths.database, &key)?;
        self.migrate_safely(&connection, &key)?;
        let settings = db::get_settings(&connection)?;
        *self
            .session
            .lock()
            .map_err(|_| AppError::Internal("Session indisponible.".into()))? =
            Some(UnlockedSession {
                database_key: key,
                last_activity: Instant::now(),
                auto_lock_minutes: settings.auto_lock_minutes,
            });
        Ok(())
    }
}
