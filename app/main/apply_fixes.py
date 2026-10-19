#!/usr/bin/env python3
"""
Script pour appliquer systématiquement toutes les corrections nécessaires
au backend pour le rendre compilable sans Tauri.
"""

import os
import re
from pathlib import Path

BASE_DIR = Path("/workspace/github__reSqur-dygcus-wumvu8__V3/app/main/src")

def fix_backend_files():
    """Appliquer les corrections à tous les fichiers backend."""
    
    backend_files = list(BASE_DIR.glob("backend/**/*.rs")) + list(BASE_DIR.glob("backend/*.rs"))
    
    for filepath in backend_files:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original = content
        
        # 1. Remplacer les doc comments module-level (!! -> //, /*! -> /**)
        # Ne pas toucher les doc comments sur les items (fonctions, structs, etc.)
        lines = content.split('\n')
        new_lines = []
        in_module_doc = False
        for i, line in enumerate(lines):
            stripped = line.strip()
            # Si on est au début du fichier et qu'on voit des doc comments
            if i == 0 and stripped.startswith('//!') or stripped.startswith('/*!'):
                # C'est un doc comment de module, remplacer
                if stripped.startswith('//!'):
                    line = line.replace('//!', '//', 1)
                elif stripped.startswith('/*!'):
                    line = line.replace('/*!', '/**', 1)
            new_lines.append(line)
        content = '\n'.join(new_lines)
        
        # 2. Remplacer #[tauri::command] par #[allow(dead_code)]
        content = content.replace('#[tauri::command]', '#[allow(dead_code)]')
        
        # 3. Remplacer tauri::State<...> par &crate::AppState
        content = re.sub(r'\btauri::State<[^>]+>\b', '&crate::AppState', content)
        
        # 4. Ajouter use crate::AppState; si pas présent
        if 'use crate::AppState;' not in content:
            # Trouver la dernière ligne d'import
            lines = content.split('\n')
            last_import_idx = -1
            for i, line in enumerate(lines):
                stripped = line.strip()
                if stripped.startswith('use ') and not stripped.startswith('//'):
                    last_import_idx = i
            
            if last_import_idx >= 0:
                # Ajouter après le dernier import
                lines.insert(last_import_idx + 1, 'use crate::AppState;')
                content = '\n'.join(lines)
            else:
                # Ajouter après la première ligne non-vide
                for i, line in enumerate(lines):
                    if line.strip() and not line.strip().startswith('//') and not line.strip().startswith('/*'):
                        lines.insert(i, 'use crate::AppState;')
                        content = '\n'.join(lines)
                        break
        
        # 5. Remplacer .lock().unwrap() par .lock().await
        content = re.sub(r'\.lock\(\)\.unwrap\(\)', '.lock().await', content)
        
        # 6. Remplacer les références directes par state.xxx
        # Mais seulement si ce n'est pas déjà state.xxx
        content = re.sub(r'\b(?!state\.)(tor_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        content = re.sub(r'\b(?!state\.)(entity_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        content = re.sub(r'\b(?!state\.)(settings)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        content = re.sub(r'\b(?!state\.)(db)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        
        # 7. Remplacer pub fn par pub async fn pour les fonctions publiques
        # Mais pas pour les fonctions déjà async
        # Utiliser une regex plus précise
        content = re.sub(r'\npub fn (\w+)', r'\npub async fn \1', content)
        
        # 8. Remplacer settings.shared.tor par settings.tor
        content = re.sub(r'\bsettings\.shared\.tor\b', 'settings.tor', content)
        content = re.sub(r'\bsettings\.shared\.maps\b', 'settings.maps', content)
        content = re.sub(r'\bsettings\.shared\.api\b', 'settings.api', content)
        content = re.sub(r'\bsettings\.shared\.watch\b', 'settings.watch', content)
        
        # 9. Remplacer self.db.lock().await par (&self.db).lock().await
        content = re.sub(r'\bself\.db\.lock\(\)\.await\b', '(&self.db).lock().await', content)
        content = re.sub(r'\bself\.tor_manager\.lock\(\)\.await\b', '(&self.tor_manager).lock().await', content)
        content = re.sub(r'\bself\.entity_manager\.lock\(\)\.await\b', '(&self.entity_manager).lock().await', content)
        content = re.sub(r'\bself\.settings\.lock\(\)\.await\b', '(&self.settings).lock().await', content)
        
        # 10. Écrire le fichier modifié
        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"Modified: {filepath}")

    print("\nBackend files processing complete")

if __name__ == "__main__":
    fix_backend_files()
