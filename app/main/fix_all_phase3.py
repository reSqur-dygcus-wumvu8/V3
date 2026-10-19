#!/usr/bin/env python3
"""
Script COMPLET pour corriger TOUTES les erreurs de compilation de la Phase 3.
Ce script applique toutes les transformations nécessaires de manière systématique.
"""

import os
import re
from pathlib import Path

BASE_DIR = Path("/workspace/github__reSqur-dygcus-wumvu8__V3/app/main/src")

def process_all_files():
    """Traiter tous les fichiers Rust du backend."""
    
    # Liste de tous les fichiers à traiter
    all_files = list(BASE_DIR.glob("backend/**/*.rs")) + list(BASE_DIR.glob("backend/*.rs"))
    
    # Étape 1: Corriger main.rs
    main_file = BASE_DIR / "main.rs"
    if main_file.exists():
        with open(main_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Remplacer les dépendances Tauri
        content = content.replace('#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]', '//')
        content = content.replace('use tauri::Manager;', 'use log::{info, error};')
        content = content.replace('use tauri::Builder;', '')
        content = content.replace('tauri::Builder::default()', '#[tokio::main]\nasync fn main() {')
        content = content.replace('fn main() {', '#[tokio::main]\nasync fn main() {')
        content = content.replace('log::', '')
        content = content.replace('tauri::generate_handler![', '// Tauri handlers: ')
        content = content.replace('tauri::generate_context!())', '// Tauri context')
        content = content.replace('.run(tauri::generate_context!()).expect(', '// Tauri run')
        content = content.replace('settings.shared.tor', 'settings.tor')
        content = content.replace('settings.shared.maps', 'settings.maps')
        content = content.replace('settings.shared.api', 'settings.api')
        content = content.replace('settings.shared.watch', 'settings.watch')
        
        # Ajouter tokio::main et async
        if '#[tokio::main]' not in content:
            content = content.replace('fn main()', '#[tokio::main]\nasync fn main()')
        
        # Ajouter le signal de fin
        if 'tokio::signal::ctrl_c' not in content:
            content = content.rstrip() + '\n    tokio::signal::ctrl_c().await.expect("Échec de l\'attente du signal");\n    info!("Arrêt de l\'application");\n}'
        
        with open(main_file, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✓ main.rs traité")
    
    # Étape 2: Traiter tous les fichiers backend
    for filepath in all_files:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
        
        original = content
        
        # 1. Doc comments - Remplacer les doc comments de module
        lines = content.split('\n')
        new_lines = []
        for i, line in enumerate(lines):
            stripped = line.strip()
            if i < 15 and (stripped.startswith('//!') or stripped.startswith('/*!')):
                if stripped.startswith('//!'):
                    line = line.replace('//!', '//', 1)
                elif stripped.startswith('/*!'):
                    line = line.replace('/*!', '/**', 1)
            new_lines.append(line)
        content = '\n'.join(new_lines)
        
        # 2. Tauri
        content = content.replace('#[tauri::command]', '#[allow(dead_code)]')
        content = re.sub(r'\btauri::State<[^>]+>\b', '&crate::AppState', content)
        
        # 3. Imports
        if 'use crate::AppState;' not in content:
            lines = content.split('\n')
            last_import_idx = -1
            for i, line in enumerate(lines):
                stripped = line.strip()
                if stripped.startswith('use ') and not stripped.startswith('//'):
                    last_import_idx = i
            if last_import_idx >= 0:
                lines.insert(last_import_idx + 1, 'use crate::AppState;')
                content = '\n'.join(lines)
        
        # 4. Mutex
        content = re.sub(r'\.lock\(\)\.unwrap\(\)', '.lock().await', content)
        
        # 5. Références directes
        content = re.sub(r'\b(?!state\.)(?!self\.)(tor_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        content = re.sub(r'\b(?!state\.)(?!self\.)(entity_manager)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        content = re.sub(r'\b(?!state\.)(?!self\.)(settings)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        content = re.sub(r'\b(?!state\.)(?!self\.)(db)\b\.lock\(\)\.await', r'state.\1.lock().await', content)
        
        # 6. Références avec self.
        content = re.sub(r'\bself\.db\.lock\(\)\.await\b', '(&self.db).lock().await', content)
        content = re.sub(r'\bself\.tor_manager\.lock\(\)\.await\b', '(&self.tor_manager).lock().await', content)
        content = re.sub(r'\bself\.entity_manager\.lock\(\)\.await\b', '(&self.entity_manager).lock().await', content)
        content = re.sub(r'\bself\.settings\.lock\(\)\.await\b', '(&self.settings).lock().await', content)
        
        # 7. Références avec state.
        content = re.sub(r'\bstate\.db\.lock\(\)\.await\b', '(&state.db).lock().await', content)
        content = re.sub(r'\bstate\.tor_manager\.lock\(\)\.await\b', '(&state.tor_manager).lock().await', content)
        content = re.sub(r'\bstate\.entity_manager\.lock\(\)\.await\b', '(&state.entity_manager).lock().await', content)
        content = re.sub(r'\bstate\.settings\.lock\(\)\.await\b', '(&state.settings).lock().await', content)
        
        # 8. Async
        content = re.sub(r'\npub fn (\w+)\s*\(', r'\npub async fn \1(', content)
        
        # 9. settings.shared
        content = re.sub(r'\bsettings\.shared\.tor\b', 'settings.tor', content)
        content = re.sub(r'\bsettings\.shared\.maps\b', 'settings.maps', content)
        content = re.sub(r'\bsettings\.shared\.api\b', 'settings.api', content)
        content = re.sub(r'\bsettings\.shared\.watch\b', 'settings.watch', content)
        
        # 10. EntityType import
        if 'EntityType' in content and 'use osint_shared::models::EntityType' not in content:
            if 'use crate::AppState;' in content:
                content = content.replace('use crate::AppState;', 
                                         'use crate::AppState;\nuse osint_shared::models::EntityType;')
        
        # 11. Corriger les ? à la fin de ligne
        content = re.sub(r'(\.\))\s*\?$', r'\1.map_err(|e| e.to_string())?', content, flags=re.MULTILINE)
        
        # 12. Corriger les appels avec state comme paramètre
        content = re.sub(r'state:\s*tauri::State<[^>]+>', 'state: &crate::AppState', content)
        
        # Écrire
        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✓ {filepath.name}")
    
    print("\n✅ Toutes les corrections appliquées")

if __name__ == "__main__":
    process_all_files()
