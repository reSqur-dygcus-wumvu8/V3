#!/bin/bash
set -e

cd /workspace/github__reSqur-dygcus-wumvu8__V3/app/main

echo "=== ÉTAPE 1: Corrections de base sûres ==="

# 1. Rendre les champs publics dans db.rs et tor.rs (déjà fait manuellement)
echo "✓ Champs publics déjà appliqués"

# 2. Remplacer les doc comments de module au début des fichiers
echo "✓ Correction des doc comments..."
find src -name "*.rs" -type f | while read file; do
    # Remplacer les 10 premières lignes seulement
    head -10 "$file" | sed 's/^\/\/! /\/\/ /g; s/^\/\*! /\/\*\* /g' > "$file.tmp"
    tail -n +11 "$file" >> "$file.tmp"
    mv "$file.tmp" "$file"
done

# 3. Remplacer #[tauri::command] par #[allow(dead_code)]
echo "✓ Remplacement des macros Tauri..."
find src/backend -name "*.rs" -exec sed -i 's/#\[tauri::command\]/#[allow(dead_code)]/g' {} \;

# 4. Remplacer tauri::State par &crate::AppState
echo "✓ Remplacement de tauri::State..."
find src/backend -name "*.rs" -exec sed -i 's/tauri::State<[^>]*>/\&crate::AppState/g' {} \;

# 5. Remplacer .lock().unwrap() par .lock().await
echo "✓ Correction des Mutex..."
find src/backend -name "*.rs" -exec sed -i 's/\.lock()\.unwrap()/\.lock().await/g' {} \;

# 6. Remplacer settings.shared.* par settings.*
echo "✓ Correction des références settings..."
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.tor/settings.tor/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.maps/settings.maps/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.api/settings.api/g' {} \;
find src/backend -name "*.rs" -exec sed -i 's/settings\.shared\.watch/settings.watch/g' {} \;

# 7. Corriger les imports EntityType
echo "✓ Correction des imports EntityType..."
find src/backend -name "*.rs" -exec sed -i 's/use crate::backend::entities::EntityType;/use osint_shared::models::EntityType;/g' {} \;

# 8. Rendre les fonctions async
echo "✓ Conversion en async..."
find src/backend -name "*.rs" -exec sed -i 's/^pub fn /pub async fn /g' {} \;

# 9. Ajouter use crate::AppState; dans tous les fichiers backend
echo "✓ Ajout des imports AppState..."
find src/backend -name "*.rs" -type f | while read file; do
    if ! grep -q "use crate::AppState;" "$file"; then
        # Trouver la dernière ligne d'import
        last_use=$(grep -n "^use " "$file" | tail -1 | cut -d: -f1)
        if [ -n "$last_use" ]; then
            sed -i "${last_use}a use crate::AppState;" "$file"
        else
            # Ajouter après la première ligne non-vide
            first_non_empty=$(grep -n "^[[:space:]]*$" "$file" -v | head -1 | cut -d: -f1)
            if [ -n "$first_non_empty" ]; then
                sed -i "${first_non_empty}i\\use crate::AppState;" "$file"
            fi
        fi
    fi
done

echo ""
echo "=== Vérification ==="
cargo check --no-default-features 2>&1 | grep "error\[" | wc -l
