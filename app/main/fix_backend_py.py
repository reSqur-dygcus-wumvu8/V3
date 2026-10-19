#!/usr/bin/env python3
"""
Script pour corriger systématiquement les erreurs de compilation du backend.
"""

import os
import re
import glob
from pathlib import Path

BASE_DIR = Path("/workspace/github__reSqur-dygcus-wumvu8__V3/app/main/src")

def fix_file(filepath):
    """Appliquer toutes les corrections à un fichier."""
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    original = content
    
    # 1. Remplacer les doc comments //! par //
    content = re.sub(r'^\s*//!', '//', content, flags=re.MULTILINE)
    content = re.sub(r'^\s*/\*!', '/*', content, flags=re.MULTILINE)
    
    # 2. Remplacer #[tauri::command] par #[allow(dead_code)]
    content = content.replace('#[tauri::command]', '#[allow(dead_code)]')
    
    # 3. Remplacer tauri::State<crate::AppState> par &crate::AppState
    content = re.sub(r'\btauri::State<crate::AppState>\b', '&crate::AppState', content)
    content = re.sub(r'\btauri::State<AppState>\b', '&crate::AppState', content)
    
    # 4. Ajouter use crate::AppState; si pas présent
    if 'use crate::AppState;' not in content:
        # Trouver la dernière ligne d'import
        lines = content.split('\n')
        last_import_idx = -1
        for i, line in enumerate(lines):
            if line.strip().startswith('use ') and not line.strip().startswith('//'):
                last_import_idx = i
        
        if last_import_idx >= 0:
            # Ajouter après le dernier import
            lines.insert(last_import_idx + 1, 'use crate::AppState;')
            content = '\n'.join(lines)
        else:
            # Ajouter au début du fichier
            content = 'use crate::AppState;\n' + content
    
    # 5. Remplacer .lock().unwrap() par .lock().await
    content = re.sub(r'\.lock\(\)\.unwrap\(\)', '.lock().await', content)
    
    # 6. Remplacer les références directes aux variables par state.xxx
    content = re.sub(r'\btor_manager\.lock\(\)\.await\b', 'state.tor_manager.lock().await', content)
    content = re.sub(r'\bentity_manager\.lock\(\)\.await\b', 'state.entity_manager.lock().await', content)
    content = re.sub(r'\bsettings\.lock\(\)\.await\b', 'state.settings.lock().await', content)
    content = re.sub(r'\bdb\.lock\(\)\.await\b', 'state.db.lock().await', content)
    
    # 7. Remplacer pub fn par pub async fn (uniquement pour les fonctions publiques)
    # Mais pas dans les impl blocks ou pour les fonctions déjà async
    content = re.sub(r'\n(pub fn \w+)', r'\npub async fn \1', content)
    
    # 8. Remplacer settings.shared.tor par settings.tor
    content = re.sub(r'\bsettings\.shared\.tor\b', 'settings.tor', content)
    content = re.sub(r'\bsettings\.shared\.maps\b', 'settings.maps', content)
    content = re.sub(r'\bsettings\.shared\.api\b', 'settings.api', content)
    content = re.sub(r'\bsettings\.shared\.watch\b', 'settings.watch', content)
    
    # 9. Remplacer state.X.lock().await par (&state.X).lock().await pour éviter les problèmes de borrow
    content = re.sub(r'state\.(\w+)\.lock\(\)\.await', r'(&state.\1).lock().await', content)
    
    # 10. Écrire le fichier modifié
    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        return True
    return False

def main():
    # Traiter tous les fichiers backend
    backend_files = glob.glob(str(BASE_DIR / "backend" / "**" / "*.rs"), recursive=True)
    
    modified_count = 0
    for filepath in backend_files:
        if fix_file(Path(filepath)):
            modified_count += 1
            print(f"Modified: {filepath}")
    
    print(f"\nTotal files modified: {modified_count}")

if __name__ == "__main__":
    main()
