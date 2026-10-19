#!/usr/bin/env python3
"""
Script complet pour corriger systématiquement toutes les erreurs de compilation
du backend pour le rendre compilable sans Tauri.
"""

import os
import re
from pathlib import Path

BASE_DIR = Path("/workspace/github__reSqur-dygcus-wumvu8__V3/app/main/src")

def process_file(filepath):
    """Traiter un fichier Rust."""
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()
    
    original = content
    
    # 1. Remplacer les doc comments de module au début des fichiers
    # Remplacer //! par // et /*! par /** au début du fichier
    lines = content.split('\n')
    new_lines = []
    for i, line in enumerate(lines):
        stripped = line.strip()
        # Si c'est un doc comment de module (au début du fichier)
        if i < 20 and (stripped.startswith('//!') or stripped.startswith('/*!')):
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
    
    # 5. Remplacer .lock().unwrap() par .lock().await
    content = re.sub(r'\.lock\(\)\.unwrap\(\)', '.lock().await', content)
    
    # 6. Remplacer les références directes aux variables par state.xxx
    # Utiliser des lookbehind négatifs pour éviter de toucher state.xxx
    content = re.sub(r'\b(?!state\.)(tor_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    content = re.sub(r'\b(?!state\.)(entity_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    content = re.sub(r'\b(?!state\.)(settings)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    content = re.sub(r'\b(?!state\.)(db)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
    
    # 7. Remplacer pub fn par pub async fn (uniquement pour les fonctions publiques non-async)
    # Ne pas toucher les fonctions déjà async
    content = re.sub(r'\npub fn (\w+)\s*\(', r'\npub async fn \1(', content)
    
    # 8. Remplacer settings.shared.* par settings.*
    content = re.sub(r'\bsettings\.shared\.tor\b', 'settings.tor', content)
    content = re.sub(r'\bsettings\.shared\.maps\b', 'settings.maps', content)
    content = re.sub(r'\bsettings\.shared\.api\b', 'settings.api', content)
    content = re.sub(r'\bsettings\.shared\.watch\b', 'settings.watch', content)
    
    # 9. Remplacer self.xxx.lock().await par (&self.xxx).lock().await
    content = re.sub(r'\bself\.db\.lock\(\)\.await\b', '(&self.db).lock().await', content)
    content = re.sub(r'\bself\.tor_manager\.lock\(\)\.await\b', '(&self.tor_manager).lock().await', content)
    content = re.sub(r'\bself\.entity_manager\.lock\(\)\.await\b', '(&self.entity_manager).lock().await', content)
    content = re.sub(r'\bself\.settings\.lock\(\)\.await\b', '(&self.settings).lock().await', content)
    
    # 10. Remplacer state: tauri::State<...> par state: &crate::AppState
    content = re.sub(r'state:\s*tauri::State<[^>]+>', 'state: &crate::AppState', content)
    
    # 11. Corriger les appels avec state comme paramètre
    # Remplacer state.X.lock().await par (&state.X).lock().await
    content = re.sub(r'state\.db\.lock\(\)\.await', '(&state.db).lock().await', content)
    content = re.sub(r'state\.tor_manager\.lock\(\)\.await', '(&state.tor_manager).lock().await', content)
    content = re.sub(r'state\.entity_manager\.lock\(\)\.await', '(&state.entity_manager).lock().await', content)
    content = re.sub(r'state\.settings\.lock\(\)\.await', '(&state.settings).lock().await', content)
    
    # Écrire le fichier modifié
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
                print(f"Modified: {filepath}")
        except Exception as e:
            print(f"Error processing {filepath}: {e}")
    
    print(f"\nTotal files modified: {modified_count}")

if __name__ == "__main__":
    main()
