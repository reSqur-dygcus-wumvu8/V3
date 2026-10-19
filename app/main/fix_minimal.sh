#!/bin/bash
set -e

cd /workspace/github__reSqur-dygcus-wumvu8__V3/app/main

echo "=== Script de correction MINIMAL et SÛR ==="
echo ""

# Étape 1: Corriger main.rs
echo "1. Correction de main.rs..."
cat > src/main.rs << 'MAINEOF'
// Application principale OSINT - Mode backend sans UI
// Gère la collecte, capitalisation et exploitation d'informations

use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, error};

// Modules backend
mod backend;
use backend::{
    db::Database,
    tor::TorManager,
    mla::MlaContext,
    entities::EntityManager,
    watch::scheduler::WatchScheduler,
};

// Module partagé
use osint_shared::models::AppSettings;

/// Structure globale de l'application
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub tor_manager: Arc<Mutex<TorManager>>,
    pub mla_context: Arc<Mutex<MlaContext>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub entity_manager: Arc<Mutex<EntityManager>>,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();
    
    info!("Démarrage de l'application OSINT (mode backend)...");
    
    let settings = AppSettings::load().expect("Échec du chargement des paramètres");
    info!("Paramètres chargés");
    
    let mla_context = MlaContext::new().expect("Échec de l'initialisation de MLA");
    info!("Contexte MLA initialisé");
    
    let db = Database::new(&settings).expect("Échec de l'initialisation de la base de données");
    info!("Base de données initialisée");
    
    let tor_manager = TorManager::new(&settings.tor).expect("Échec de l'initialisation de TOR");
    info!("Gestionnaire TOR initialisé");
    
    let entity_manager = EntityManager::new(Arc::new(Mutex::new(db.clone())));
    
    let app_state = Arc::new(AppState {
        db: Arc::new(Mutex::new(db)),
        tor_manager: Arc::new(Mutex::new(tor_manager)),
        mla_context: Arc::new(Mutex::new(mla_context)),
        settings: Arc::new(Mutex::new(settings)),
        entity_manager: Arc::new(Mutex::new(entity_manager)),
    });
    
    let db_clone = Arc::clone(&app_state.db);
    let tor_clone = Arc::clone(&app_state.tor_manager);
    let settings_clone = Arc::clone(&app_state.settings);
    tokio::spawn(async move {
        let mut watch_scheduler = WatchScheduler::new(db_clone, tor_clone, settings_clone);
        watch_scheduler.start().await;
    });
    
    if app_state.settings.lock().await.tor.auto_start {
        let tor_manager_clone = Arc::clone(&app_state.tor_manager);
        tokio::spawn(async move {
            if let Err(e) = tor_manager_clone.lock().await.start_tor() {
                error!("Échec du démarrage de TOR: {}", e);
            } else {
                info!("TOR démarré avec succès");
            }
        });
    }
    
    info!("Application OSINT prête - Mode backend (sans UI)");
    tokio::signal::ctrl_c().await.expect("Échec de l'attente du signal");
    info!("Arrêt de l'application");
}
MAINEOF

# Étape 2: Rendre les champs publics
echo "2. Rendre les champs publics..."
sed -i 's/    conn: Connection,/    pub conn: Connection,/' src/backend/db.rs
sed -i 's/    path: PathBuf,/    pub path: PathBuf,/' src/backend/db.rs
sed -i 's/    mla_enabled: bool,/    pub mla_enabled: bool,/' src/backend/db.rs

sed -i 's/    client: Client,/    pub client: Client,/' src/backend/tor.rs
sed -i 's/    socks_port: u16,/    pub socks_port: u16,/' src/backend/tor.rs

# Étape 3: Remplacer tauri::command
echo "3. Remplacer tauri::command..."
find src/backend -name "*.rs" -exec sed -i 's/#\[tauri::command\]/#[allow(dead_code)]/g' {} \;

# Étape 4: Remplacer tauri::State
echo "4. Remplacer tauri::State..."
find src/backend -name "*.rs" -exec sed -i 's/tauri::State<[^>]*>/\&crate::AppState/g' {} \;

# Étape 5: Remplacer .lock().unwrap() par .lock().await
echo "5. Corriger Mutex..."
find src/backend -name "*.rs" -exec sed -i 's/\.lock()\.unwrap()/\.lock().await/g' {} \;

# Étape 6: Remplacer settings.shared.* par settings.*
echo "6. Corriger références settings..."
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.tor/settings.tor/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.maps/settings.maps/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.api/settings.api/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.watch/settings.watch/g' {} \;

# Étape 7: Ajouter use crate::AppState
echo "7. Ajouter imports AppState..."
find src/backend -name "*.rs" -type f | while read file; do
    if ! grep -q "use crate::AppState;" "$file"; then
        last_use=$(grep -n "^use " "$file" | tail -1 | cut -d: -f1)
        if [ -n "$last_use" ]; then
            sed -i "${last_use}a use crate::AppState;" "$file"
        fi
    fi
done

# Étape 8: Rendre les fonctions async
echo "8. Rendre les fonctions async..."
find src/backend -name "*.rs" -exec sed -i 's/^pub fn /pub async fn /g' {} \;

echo ""
echo "=== Vérification ==="
cargo check --no-default-features 2>&1 | grep "error\[" | wc -l
