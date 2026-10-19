#!/bin/bash
set -e

cd /workspace/github__reSqur-dygcus-wumvu8__V3/app/main

echo "=== Étape 1: Remplacer les doc comments ==="
find src -name "*.rs" -exec sed -i 's/^\/\/! /\/\/ /g' {} \;
find src -name "*.rs" -exec sed -i 's/^\/\*! /\/\*\* /g' {} \;

echo "=== Étape 2: Remplacer tauri::command ==="
find src/backend -name "*.rs" -exec sed -i 's/#\[tauri::command\]/#[allow(dead_code)]/g' {} \;

echo "=== Étape 3: Remplacer tauri::State ==="
find src/backend -name "*.rs" -exec sed -i 's/tauri::State<crate::AppState>/\&crate::AppState/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/tauri::State<AppState>/\&crate::AppState/g' {} \;

echo "=== Étape 4: Ajouter use crate::AppState ==="
for file in $(find src/backend -name "*.rs" -type f); do
    if ! grep -q "use crate::AppState;" "$file"; then
        # Ajouter après le dernier use existant
        sed -i '/^use / { /^$/! { s/^use .*/&\nuse crate::AppState; / } }' "$file"
        # Si pas de use du tout, ajouter au début
        if ! grep -q "use crate::AppState;" "$file"; then
            sed -i '1i\\nuse crate::AppState;' "$file"
        fi
    fi
done

echo "=== Étape 5: Remplacer .lock().unwrap() par .lock().await ==="
find src/backend -name "*.rs" -exec sed -i 's/\.lock()\.unwrap()/\.lock().await/g' {} \;

echo "=== Étape 6: Remplacer variables directes par state.xxx ==="
find src/backend -name "*.rs" -exec sed -i 's/\<tor_manager\.lock()\.await/state.tor_manager.lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/\<entity_manager\.lock()\.await/state.entity_manager.lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/\<settings\.lock()\.await/state.settings.lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/\<db\.lock()\.await/state.db.lock().await/g' {} \;

echo "=== Étape 7: Rendre les fonctions async ==="
# Remplacer pub fn par pub async fn dans les fichiers backend
find src/backend -name "*.rs" -exec sed -i 's/^pub fn /pub async fn /g' {} \;

echo "=== Étape 8: Fix main.rs ==="
# Remplacer les références à settings.shared.tor par settings.tor
sed -i 's/settings\.shared\.tor/settings\.tor/g' src/main.rs
sed -i 's/settings\.shared\.maps/settings\.maps/g' src/main.rs
sed -i 's/settings\.shared\.api/settings\.api/g' src/main.rs
sed -i 's/settings\.shared\.watch/settings\.watch/g' src/main.rs

# Ajouter l'import TorSettings
sed -i 's/use osint_shared::models::AppSettings;/use osint_shared::models::{AppSettings, TorSettings, MapSettings, ApiSettings, WatchSettings};/g' src/main.rs

echo "=== Compilation test ==="
cargo check --no-default-features 2>&1 | grep "error\[" | wc -l
