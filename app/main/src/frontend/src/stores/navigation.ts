// Store pour la navigation
import { writable } from 'svelte/store';

export const currentPage = writable<string>('/');

export const breadcrumbs = writable<Array<{ name: string; path: string }>>([]);

// Mettre à jour les breadcrumbs
export function updateBreadcrumbs(path: string) {
    const parts = path.split('/').filter(p => p !== '');
    const newBreadcrumbs: Array<{ name: string; path: string }> = [
        { name: 'Accueil', path: '/' },
    ];
    
    let currentPath = '';
    for (const part of parts) {
        currentPath += `/${part}`;
        const name = formatBreadcrumbName(part);
        newBreadcrumbs.push({ name, path: currentPath });
    }
    
    breadcrumbs.set(newBreadcrumbs);
}

function formatBreadcrumbName(part: string): string {
    const mappings: Record<string, string> = {
        'acquerir': 'Acquérir',
        'capitaliser': 'Capitaliser',
        'exploiter': 'Exploiter',
        'gerer': 'Gérer',
        'import': 'Import',
        'watch': 'Veille',
        'discovery': 'Découverte',
        'entities': 'Entités',
        'graph': 'Graphe',
        'timeline': 'Frise',
        'map': 'Carte',
        'settings': 'Paramètres',
        'duplicates': 'Doublons',
    };
    
    return mappings[part] || part.charAt(0).toUpperCase() + part.slice(1);
}
