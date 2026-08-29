use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("Session verrouillée. Saisissez votre PIN pour continuer.")]
    Locked,
    #[error("PIN incorrect ou données de sécurité invalides.")]
    InvalidPin,
    #[error("Mot de passe de récupération incorrect.")]
    InvalidRecoveryPassword,
    #[error("Élément introuvable.")]
    NotFound,
    #[error("Erreur de base de données: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Erreur de fichier: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erreur de sécurité: {0}")]
    Security(String),
    #[error("Erreur d’export: {0}")]
    Export(String),
    #[error("Erreur interne: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(value: csv::Error) -> Self {
        Self::Export(value.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::Internal(value.to_string())
    }
}
