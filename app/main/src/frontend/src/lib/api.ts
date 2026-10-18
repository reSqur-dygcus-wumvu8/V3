// API client pour les appels au backend Tauri

import { invoke } from '@tauri-apps/api/tauri';
import type {
    Entity,
    Watch,
    WatchResult,
    CapitalizedInfo,
    EntityGraph,
    TimelineEvent,
    AcledEvent,
    DuplicateGroup,
    TorStatus,
    TorExitNode,
    AppSettings,
} from '$types';

// ========== Gestion des paramètres ==========

export async function getSettings(): Promise<AppSettings> {
    return await invoke('get_settings');
}

export async function saveSettings(settings: AppSettings): Promise<void> {
    await invoke('save_settings', { settings });
}

// ========== Gestion TOR ==========

export async function getTorStatus(): Promise<TorStatus> {
    return await invoke('get_tor_status');
}

export async function getTorExitNodes(): Promise<TorExitNode[]> {
    return await invoke('get_tor_exit_nodes');
}

export async function testTorConnection(): Promise<boolean> {
    return await invoke('test_tor_connection');
}

export async function refreshTorExitNodes(): Promise<TorExitNode[]> {
    return await invoke('refresh_tor_exit_nodes');
}

// ========== Gestion de la base de données ==========

export async function testDbConnection(): Promise<boolean> {
    return await invoke('test_db_connection');
}

// ========== Import de fichiers ==========

export interface ImportFileParams {
    name: string;
    data: Uint8Array;
    sourceRating: string;
    date: string;
}

export async function importFile(params: ImportFileParams): Promise<void> {
    await invoke('import_file', params);
}

// ========== Veille Internet ==========

export async function getWatches(): Promise<Watch[]> {
    return await invoke('get_watches');
}

export interface AddWatchParams {
    name: string;
    query: string;
    language: string;
    sources: string[];
    frequency: number;
    isActive: boolean;
}

export async function addWatch(params: AddWatchParams): Promise<number> {
    return await invoke('add_watch', { watch: params });
}

export interface UpdateWatchParams {
    id: number;
    isActive?: boolean;
    name?: string;
    query?: string;
    language?: string;
    sources?: string[];
    frequency?: number;
}

export async function updateWatch(params: UpdateWatchParams): Promise<void> {
    await invoke('update_watch', params);
}

export async function deleteWatch(id: number): Promise<void> {
    await invoke('delete_watch', { id });
}

export async function getWatchResults(watchId: number): Promise<WatchResult[]> {
    return await invoke('get_watch_results', { watchId });
}

// ========== Mode Découverte ==========

export interface DiscoveryParams {
    topic: string;
}

export interface DiscoveryResult {
    summary: string;
    sources: Array<{ name: string; url: string }>;
    keywords: string[];
}

export async function discoverTopic(params: DiscoveryParams): Promise<DiscoveryResult> {
    return await invoke('discover_topic', params);
}

// ========== Capitalisation ==========

export async function getProcessedWatchResults(): Promise<WatchResult[]> {
    return await invoke('get_processed_watch_results');
}

export interface CapitalizeInfoParams {
    watchResultId: number;
    entityType: string;
    entityId: number;
    cotation: {
        reliability: string;
        credibility: string;
    };
}

export async function capitalizeInfo(params: CapitalizeInfoParams): Promise<number> {
    return await invoke('capitalize_info', params);
}

// ========== Exploitation ==========

export async function getEntity(entityType: string, id: number): Promise<Entity | null> {
    try {
        return await invoke('get_entity', { entityType, id });
    } catch (error) {
        console.error('Erreur lors de la récupération de l\'entité:', error);
        return null;
    }
}

export interface GetEntityGraphParams {
    id: number;
    type: string;
    degrees: number;
}

export async function getEntityGraph(params: GetEntityGraphParams): Promise<EntityGraph> {
    return await invoke('get_entity_graph', params);
}

export async function getEntityTimeline(id: number, type: string): Promise<TimelineEvent[]> {
    return await invoke('get_entity_timeline', { id, type });
}

// ========== Gestion des doublons ==========

export async function getDuplicateGroups(): Promise<DuplicateGroup[]> {
    return await invoke('get_duplicate_groups');
}

export interface ResolveDuplicateParams {
    groupId: number;
    resolution: 'merge' | 'ignore' | 'separate';
    mainEntityId?: number;
}

export async function resolveDuplicate(params: ResolveDuplicateParams): Promise<void> {
    await invoke('resolve_duplicate', params);
}

// ========== Utilitaires ==========

// Générer une clé de session (à implémenter avec l'app Secrets)
export async function generateSessionKey(context: string): Promise<string> {
    // Cela nécessitera une communication avec l'app Secrets
    // Pour l'instant, on retourne une clé de test
    return 'test-session-key-' + context;
}

// Vérifier la connexion Internet (via TOR)
export async function checkInternetConnection(): Promise<boolean> {
    try {
        const status = await getTorStatus();
        if (!status.isRunning) return false;
        return await testTorConnection();
    } catch (error) {
        return false;
    }
}
