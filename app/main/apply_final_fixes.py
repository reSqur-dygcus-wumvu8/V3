#!/usr/bin/env python3
"""
Script final pour corriger TOUTES les erreurs de compilation du backend.
Approche systématique et complète.
"""

import os
import re
from pathlib import Path

BASE_DIR = Path("/workspace/github__reSqur-dygcus-wumvu8__V3/app/main/src")

def process_file(filepath):
    """Traiter un fichier Rust de manière complète."""
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()
    
    original = content
    
    # ========================================================================
    # ÉTAPE 1: Doc comments - Remplacer les doc comments de module
    # ========================================================================
    lines = content.split('\n')
    new_lines = []
    in_module_doc = True  # Au début du fichier
    for i, line in enumerate(lines):
        stripped = line.strip()
        # Si on est dans la zone de doc comment de module (début de fichier)
        if in_module_doc and (stripped.startswith('//!') or stripped.startswith('/*!')):
            if stripped.startswith('//!'):
                line = line.replace('//!', '//', 1)
            elif stripped.startswith('/*!'):
                line = line.replace('/*!', '/**', 1)
        # Si on trouve une ligne non-doc et non-vide, on sort de la zone module doc
        elif in_module_doc and stripped and not stripped.startswith('//') and not stripped.startswith('/*') and not stripped.startswith('///') and not stripped.startswith('/**'):
            in_module_doc = False
        new_lines.append(line)
    content = '\n'.join(new_lines)
    
    # ========================================================================
    # ÉTAPE 2: Tauri - Remplacer toutes les références Tauri
    # ========================================================================
    # Remplacer #[tauri::command] par #[allow(dead_code)]
    content = content.replace('#[tauri::command]', '#[allow(dead_code)]')
    
    # Remplacer tauri::State<...> par &crate::AppState
    content = re.sub(r'\btauri::State<[^>]+>\b', '&crate::AppState', content)
    
    # ========================================================================
    # ÉTAPE 3: Imports - Ajouter les imports nécessaires
    # ========================================================================
    # Ajouter use crate::AppState; si pas présent
    if 'use crate::AppState;' not in content:
        lines = content.split('\n')
        last_import_idx = -1
        for i, line in enumerate(lines):
            stripped = line.strip()
            if stripped.startswith('use ') and not stripped.startswith('//'):
                last_import_idx = i
        
        if last_import_idx >= 0:
            # Vérifier si la ligne suivante est vide ou un commentaire
            if last_import_idx + 1 < len(lines):
                next_stripped = lines[last_import_idx + 1].strip()
                if not next_stripped or next_stripped.startswith('//'):
                    lines.insert(last_import_idx + 1, 'use crate::AppState;')
                else:
                    lines.insert(last_import_idx + 1, '')
                    lines.insert(last_import_idx + 2, 'use crate::AppState;')
            else:
                lines.append('')
                lines.append('use crate::AppState;')
            content = '\n'.join(lines)
        else:
            # Ajouter après la première ligne non-vide
            for i, line in enumerate(lines):
                if line.strip() and not line.strip().startswith('//') and not line.strip().startswith('/*'):
                    lines.insert(i, 'use crate::AppState;')
                    content = '\n'.join(lines)
                    break
    
    # Ajouter EntityType import si le fichier l'utilise
    if 'EntityType' in content and 'use crate::backend::entities::EntityType' not in content:
        if 'use crate::AppState;' in content:
            content = content.replace('use crate::AppState;', 
                                     'use crate::AppState;\nuse crate::backend::entities::EntityType;')
        else:
            # Trouver un bon endroit pour l'ajouter
            if 'use ' in content:
                content = content.replace('use ', 'use crate::backend::entities::EntityType;\nuse ', 1)
    
    # ========================================================================
    # ÉTAPE 4: Mutex - Remplacer .lock().unwrap() par .lock().await
    # ========================================================================
    content = re.sub(r'\.lock\(\)\.unwrap\(\)', '.lock().await', content)
    
    # ========================================================================
    # ÉTAPE 5: Références - Remplacer les références directes
    # ========================================================================
    # Remplacer les références directes (tor_manager, entity_manager, settings, db)
    # par state.X.lock().await OU (&self.X).lock().await selon le contexte
    
    # Pour les références directes (sans self. ni state.)
    content = re.sub(r'\b(?!state\.)(?!self\.)(tor_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    content = re.sub(r'\b(?!state\.)(?!self\.)(entity_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    content = re.sub(r'\b(?!state\.)(?!self\.)(settings)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    content = re.sub(r'\b(?!state\.)(?!self\.)(db)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    
    # Pour les références avec self.
    content = re.sub(r'\bself\.db\.lock\(\)\.await\b', '(&self.db).lock().await', content)
    content = re.sub(r'\bself\.tor_manager\.lock\(\)\.await\b', '(&self.tor_manager).lock().await', content)
    content = re.sub(r'\bself\.entity_manager\.lock\(\)\.await\b', '(&self.entity_manager).lock().await', content)
    content = re.sub(r'\bself\.settings\.lock\(\)\.await\b', '(&self.settings).lock().await', content)
    
    # Pour les références avec state.
    content = re.sub(r'\bstate\.db\.lock\(\)\.await\b', '(&state.db).lock().await', content)
    content = re.sub(r'\bstate\.tor_manager\.lock\(\)\.await\b', '(&state.tor_manager).lock().await', content)
    content = re.sub(r'\bstate\.entity_manager\.lock\(\)\.await\b', '(&state.entity_manager).lock().await', content)
    content = re.sub(r'\bstate\.settings\.lock\(\)\.await\b', '(&state.settings).lock().await', content)
    
    # ========================================================================
    # ÉTAPE 6: Async - Rendre les fonctions async
    # ========================================================================
    # Remplacer pub fn par pub async fn (uniquement si ce n'est pas déjà async)
    # Ne pas toucher les impl blocks ou les fonctions déjà async
    content = re.sub(r'\npub fn (\w+)\s*\(', r'\npub async fn \1(', content)
    
    # ========================================================================
    # ÉTAPE 7: settings.shared.* -> settings.*
    # ========================================================================
    content = re.sub(r'\bsettings\.shared\.tor\b', 'settings.tor', content)
    content = re.sub(r'\bsettings\.shared\.maps\b', 'settings.maps', content)
    content = re.sub(r'\bsettings\.shared\.api\b', 'settings.api', content)
    content = re.sub(r'\bsettings\.shared\.watch\b', 'settings.watch', content)
    
    # ========================================================================
    # ÉTAPE 8: Signatures - Corriger state: tauri::State -> state: &crate::AppState
    # ========================================================================
    content = re.sub(r'state:\s*tauri::State<[^>]+>', 'state: &crate::AppState', content)
    
    # ========================================================================
    # Écrire le fichier modifié
    # ========================================================================
    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        return True
    return False

def main():
    # Traiter tous les fichiers backend
    backend_files = list(BASE_DIR.glob("backend/**/*.rs")) + list(BASE_DIR.glob("backend/*.rs"))
    
    modified_count = 0
    for filepath in backend_files:
        try:
            if process_file(filepath):
                modified_count += 1
                print(f"✓ {filepath}")
        except Exception as e:
            print(f"✗ {filepath}: {e}")
    
    print(f"\n✅ Total files modified: {modified_count}")

if __name__ == "__main__":
    main()
