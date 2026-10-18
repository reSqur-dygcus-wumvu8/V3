//! Constantes partagées

/// Version de l'application
pub const APP_VERSION: &str = "0.1.0";

/// Nom de l'application principale
pub const MAIN_APP_NAME: &str = "OSINT App";

/// Nom de l'application Secrets
pub const SECRETS_APP_NAME: &str = "OSINT Secrets";

/// Identifiant de l'organisation
pub const ORG_ID: &str = "com.yourorg";

/// Chemin de configuration par défaut
pub const CONFIG_DIR: &str = "osint_app";

/// Port SOCKS5 par défaut pour TOR
pub const DEFAULT_TOR_SOCKS_PORT: u16 = 9050;

/// Port de contrôle par défaut pour TOR
pub const DEFAULT_TOR_CONTROL_PORT: u16 = 9051;

/// Timeout par défaut pour les requêtes HTTP (en secondes)
pub const DEFAULT_HTTP_TIMEOUT: u64 = 30;

/// Taille maximale du cache des tuiles (en Mo)
pub const MAX_TILE_CACHE_SIZE: u32 = 1000;

/// Fréquence de sauvegarde automatique (en minutes)
pub const AUTO_SAVE_FREQUENCY: u64 = 5;

/// Nombre maximal de résultats de veille par requête
pub const MAX_WATCH_RESULTS: u32 = 100;

/// Longueur maximale des champs texte
pub const MAX_TEXT_FIELD_LENGTH: usize = 10000;

/// Longueur maximale des noms
pub const MAX_NAME_FIELD_LENGTH: usize = 255;
