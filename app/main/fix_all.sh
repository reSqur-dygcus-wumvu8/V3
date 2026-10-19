#!/bin/bash
set -e

cd /workspace/github__reSqur-dygcus-wumvu8__V3/app/main

echo "=== Application des corrections systématiques ==="

# 1. Remplacer les doc comments de module
find src -name "*.rs" -type f | while read file; do
    # Remplacer les doc comments de module au début des fichiers
    sed -i '1,10s/^\/\/! /\/\/ /g' "$file"
    sed -i '1,10s/^\/\*! /\/\*\* /g' "$file"
done

# 2. Remplacer tauri::command
find src/backend -name "*.rs" -exec sed -i 's/#\[tauri::command\]/#[allow(dead_code)]/g' {} \;

# 3. Remplacer tauri::State
find src/backend -name "*.rs" -exec sed -i 's/tauri::State<crate::AppState>/\&crate::AppState/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/tauri::State<AppState>/\&crate::AppState/g' {} \;

# 4. Ajouter use crate::AppState
find src/backend -name "*.rs" -type f | while read file; do
    if ! grep -q "use crate::AppState;" "$file"; then
        # Trouver la dernière ligne d'import et ajouter après
        if grep -q "^use " "$file"; then
            # Ajouter après le dernier use
            sed -i '/^use / { /^$/! { s/^use .*/&\nuse crate::AppState; / } }' "$file"
        else
            # Ajouter au début du fichier
            sed -i '1i\\nuse crate::AppState;' "$file"
        fi
    fi
done

# 5. Remplacer .lock().unwrap() par .lock().await
find src/backend -name "*.rs" -exec sed -i 's/\.lock()\.unwrap()/\.lock().await/g' {} \;

# 6. Remplacer les références directes par state.xxx
find src/backend -name "*.rs" -exec sed -i 's/\<tor_manager\.lock()\.await/state.tor_manager.lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/\<entity_manager\.lock()\.await/state.entity_manager.lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/\<settings\.lock()\.await/state.settings.lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/\<db\.lock()\.await/state.db.lock().await/g' {} \;

# 7. Remplacer pub fn par pub async fn
find src/backend -name "*.rs" -exec sed -i 's/^pub fn /pub async fn /g' {} \;

# 8. Remplacer settings.shared.* par settings.*
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.tor/settings.tor/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.maps/settings.maps/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.api/settings.api/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.watch/settings.watch/g' {} \;

# 9. Remplacer self.xxx.lock().await par (&self.xxx).lock().await
find src/backend -name "*.rs" -exec sed -i 's/self\.db\.lock()\.await/(&self.db).lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/self\.tor_manager\.lock()\.await/(&self.tor_manager).lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/self\.entity_manager\.lock()\.await/(&self.entity_manager).lock().await/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/self\.settings\.lock()\.await/(&self.settings).lock().await/g' {} \;

echo "=== Corrections de base appliquées ==="
echo ""
echo "Vérification initiale:"
cargo check --no-default-features 2>&1 | grep "error\[" | wc -l
