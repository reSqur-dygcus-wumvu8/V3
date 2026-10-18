// Utilitaires pour l'application OSINT

import { Cotation } from '$types';

// Formatage des dates
export function formatDate(dateString: string | undefined): string {
    if (!dateString) return 'Non spécifiée';
    
    const date = new Date(dateString);
    if (isNaN(date.getTime())) return dateString;
    
    return date.toLocaleDateString('fr-FR', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
    });
}

// Formatage des cotations OTAN
export function formatCotation(cotation: Cotation | string): string {
    if (typeof cotation === 'string') {
        return cotation;
    }
    return `${cotation.reliability}${cotation.credibility}`;
}

// Obtenir le libellé de la fiabilité
export function getReliabilityLabel(code: string): string {
    const labels: Record<string, string> = {
        'A': 'Totalement fiable',
        'B': 'Généralement fiable',
        'C': 'Assez fiable',
        'D': 'Peu fiable',
        'E': 'Fiabilité non évaluable',
        'F': 'Non évaluable',
    };
    return labels[code] || code;
}

// Obtenir le libellé de la crédibilité
export function getCredibilityLabel(code: string): string {
    const labels: Record<string, string> = {
        '1': 'Confirmée',
        '2': 'Probablement vraie',
        '3': 'Possiblement vraie',
        '4': 'Douteuse',
        '5': 'Improbable',
        '6': 'Non évaluable',
    };
    return labels[code] || code;
}

// Obtenir la couleur de la fiabilité
export function getReliabilityColor(code: string): string {
    const colors: Record<string, string> = {
        'A': 'bg-green-500',
        'B': 'bg-blue-500',
        'C': 'bg-yellow-500',
        'D': 'bg-orange-500',
        'E': 'bg-red-500',
        'F': 'bg-gray-500',
    };
    return colors[code] || 'bg-gray-500';
}

// Obtenir la couleur de la crédibilité
export function getCredibilityColor(code: string): string {
    const colors: Record<string, string> = {
        '1': 'bg-green-500',
        '2': 'bg-blue-500',
        '3': 'bg-yellow-500',
        '4': 'bg-orange-500',
        '5': 'bg-red-500',
        '6': 'bg-gray-500',
    };
    return colors[code] || 'bg-gray-500';
}

// Générer un UUID v4
export function generateUUID(): string {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
        const r = (Math.random() * 16) | 0;
        const v = c === 'x' ? r : (r & 0x3) | 0x8;
        return v.toString(16);
    });
}

// Calculer le hash SHA-256 (pour la déduplication)
export async function calculateSHA256(file: File): Promise<string> {
    const arrayBuffer = await file.arrayBuffer();
    const hashBuffer = await crypto.subtle.digest('SHA-256', arrayBuffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

// Tronquer un texte
export function truncateText(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength) + '...';
}

// Formater un nombre
export function formatNumber(num: number | undefined): string {
    if (num === undefined) return '0';
    return new Intl.NumberFormat('fr-FR').format(num);
}

// Vérifier si un texte contient des entités à détecter
export function containsEntities(text: string): boolean {
    // Regex pour détecter des noms propres, dates, etc.
    const patterns = [
        /\b[A-Z][a-z]+ [A-Z][a-z]+\b/, // Noms propres (ex: John Doe)
        /\b\d{1,2}[/-]\d{1,2}[/-]\d{2,4}\b/, // Dates
        /\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b/, // Adresses IP
        /\b[A-Z][a-z]+\b/, // Mots en majuscule
    ];
    
    return patterns.some(pattern => pattern.test(text));
}

// Générer un nom de fichier unique
export function generateUniqueFilename(originalName: string): string {
    const timestamp = Date.now();
    const random = Math.random().toString(36).substring(2, 8);
    const extension = originalName.split('.').pop();
    const baseName = originalName.replace(`.${extension}`, '');
    return `${baseName}_${timestamp}_${random}.${extension}`;
}
