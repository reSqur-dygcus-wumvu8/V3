#!/usr/bin/env python3
"""
Script de nettoyage pour corriger les erreurs de syntaxe introduites par les scripts précédents.
"""

import re
from pathlib import Path

BASE_DIR = Path("/workspace/github__reSqur-dygcus-wumvu8__V3/app/main/src")

def clean_file(filepath):
    """Nettoyer un fichier des erreurs de syntaxe."""
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()
    
    original = content
    
    # Corriger les patterns malformés créés par les scripts précédents
    
    # 1. Corriger self.(&state.X) -> (&self.X)
    content = re.sub(r'self\.\(\&state\.(\w+)\)', r'(&self.\1)', content)
    
    # 2. Corriger self.(&state.self.X) -> (&self.X)
    content = re.sub(r'self\.\(\&state\.self\.(\w+)\)', r'(&self.\1)', content)
    
    # 3. Corriger self.(&state.self.self.X) -> (&self.X)
    content = re.sub(r'self\.\(\&state\.self\.self\.(\w+)\)', r'(&self.\1)', content)
    
    # 4. Corriger les doubles parenthèses ((self.X) -> (&self.X)
    content = re.sub(r'\(\(&self\.(\w+)\)', r'(&self.\1)', content)
    
    # 5. Corriger les imports en double
    # Supprimer les doublons de use crate::AppState;
    lines = content.split('\n')
    new_lines = []
    seen_imports = set()
    for line in lines:
        stripped = line.strip()
        if stripped == 'use crate::AppState;' and stripped in seen_imports:
            continue
        elif stripped == 'use crate::AppState;':
            seen_imports.add(stripped)
        new_lines.append(line)
    content = '\n'.join(new_lines)
    
    # 6. Corriger les imports EntityType en double
    lines = content.split('\n')
    new_lines = []
    seen_entitytype = False
    for line in lines:
        stripped = line.strip()
        if 'EntityType' in stripped and 'use' in stripped:
            if seen_entitytype:
                continue
            seen_entitytype = True
        new_lines.append(line)
    content = '\n'.join(new_lines)
    
    # 7. Corriger les doc comments mal convertis
    content = re.sub(r'^\s*///', '//', content, flags=re.MULTILINE)
    
    # Écrire le fichier nettoyé
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
            if clean_file(filepath):
                modified_count += 1
                print(f"✓ Cleaned: {filepath}")
        except Exception as e:
            print(f"✗ Error cleaning {filepath}: {e}")
    
    print(f"\n✅ Total files cleaned: {modified_count}")

if __name__ == "__main__":
    main()
