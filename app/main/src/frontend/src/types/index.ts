// Types TypeScript pour l'application OSINT

// Types d'entités
export type EntityType = 'individual' | 'organization' | 'event' | 'location' | 'object' | 'source';

// Statut de validation
export type ValidationStatus = 'pending' | 'validated' | 'rejected';

// Cotation OTAN
export interface Cotation {
    reliability: 'A' | 'B' | 'C' | 'D' | 'E' | 'F';
    credibility: '1' | '2' | '3' | '4' | '5' | '6';
}

// Entité générique
export interface Entity {
    id: number;
    type: EntityType;
    name: string;
    createdAt: string;
    updatedAt: string;
    validated: boolean;
    comments?: string;
}

// Individu
export interface Individual extends Entity {
    type: 'individual';
    individualType: 'real' | 'fictive' | 'digital';
    firstName?: string;
    lastName?: string;
    biography?: string;
    wikipediaUrl?: string;
    linkedRealIdentities?: number[]; // IDs des identités réelles liées (pour fictif/numérique)
    socialAccounts?: SocialAccount[];
}

// Organisation
export interface Organization extends Entity {
    type: 'organization';
    organizationType: string;
    summary?: string;
}

// Événement
export interface Event extends Entity {
    type: 'event';
    startDate?: string;
    endDate?: string;
    summary?: string;
}

// Lieu
export interface Location extends Entity {
    type: 'location';
    locationType: 'real' | 'fictive' | 'digital';
    address?: string;
    gpsLat?: number;
    gpsLon?: number;
    ipAddress?: string;
    summary?: string;
}

// Objet
export interface Object extends Entity {
    type: 'object';
    objectType: string;
    details?: string;
}

// Source
export interface Source extends Entity {
    type: 'source';
    description?: string;
    url?: string;
    reliability?: 'A' | 'B' | 'C' | 'D' | 'E' | 'F';
}

// Compte réseau social
export interface SocialAccount {
    id: number;
    platform: string;
    username: string;
    url?: string;
    verified: boolean;
}

// Veille
export interface Watch {
    id: number;
    name: string;
    query: string;
    language: string;
    sources: string[]; // Ex: ['google_news', 'qwant']
    frequencyMinutes: number;
    isActive: boolean;
    createdAt: string;
    updatedAt: string;
}

// Résultat de veille
export interface WatchResult {
    id: number;
    watchId: number;
    title: string;
    url: string;
    content: string;
    translatedContent?: string;
    sourceName: string;
    publishedAt?: string;
    fetchedAt: string;
    isProcessed: boolean;
    metadata?: any;
    entities?: EntityReference[];
}

// Référence à une entité
export interface EntityReference {
    entityType: EntityType;
    entityId: number;
    name: string;
    confidence: number; // 0-1
}

// Information capitalisée
export interface CapitalizedInfo {
    id: number;
    title: string;
    content: string;
    eventId?: number;
    sourceId: number;
    watchResultId?: number;
    fileId?: number;
    createdAt: string;
    validated: boolean;
    entities?: EntityReference[];
    cotation?: Cotation;
}

// Relation entre entités
export interface EntityRelation {
    id: number;
    fromType: EntityType;
    fromId: number;
    fromName?: string;
    toType: EntityType;
    toId: number;
    toName?: string;
    relationType: string;
    description?: string;
    sourceId?: number;
    createdAt: string;
}

// Graphe d'entités
export interface EntityGraph {
    nodes: GraphNode[];
    edges: GraphEdge[];
}

export interface GraphNode {
    id: string;
    label: string;
    type: EntityType;
    degree: number;
    color?: string;
    shape?: string;
}

export interface GraphEdge {
    id: string;
    source: string;
    target: string;
    label: string;
}

// Événement pour la frise chronologique
export interface TimelineEvent {
    id: number;
    name: string;
    summary?: string;
    startDate: string;
    endDate?: string;
    source?: string;
    reliability?: string;
    credibility?: string;
    eventType: EntityType;
}

// Événement ACLED
export interface AcledEvent {
    eventId: string;
    eventDate: string;
    eventType: string;
    latitude?: number;
    longitude?: number;
    location: string;
    actor1: string;
    actor2: string;
    fatalities?: number;
    reliability: string;
    credibility: string;
}

// Groupe de doublons
export interface DuplicateGroup {
    id: number;
    entities: EntityReference[];
    similarityScore: number; // 0-1
    status: 'pending' | 'resolved' | 'ignored' | 'merged';
    resolutionType?: string;
}

// Paramètres de l'application
export interface AppSettings {
    tor: {
        enabled: boolean;
        socksPort: number;
        controlPort: number;
        autoStart: boolean;
        exitNodes: string[];
    };
    maps: {
        defaultProvider: string;
        cacheEnabled: boolean;
        cacheSize: number;
    };
    api: {
        mistralKey?: string;
        mistralEndpoint: string;
        aclEndpoint: string;
    };
    watch: {
        defaultFrequency: number;
        defaultLanguage: string;
        maxConcurrentWatches: number;
    };
}

// Statut TOR
export interface TorStatus {
    isRunning: boolean;
    version?: string;
    socksPort: number;
    controlPort: number;
    bootstrapStatus?: string;
    circuits: number;
    streams: number;
}

// Nœud de sortie TOR
export interface TorExitNode {
    nickname: string;
    fingerprint: string;
    ip: string;
    country: string;
    countryCode: string;
    isRunning: boolean;
    lastSeen?: string;
}

// Statut de l'application
export interface AppStatus {
    tor: TorStatus;
    db: {
        isEncrypted: boolean;
        lastBackup?: string;
    };
    lastSync: string;
}
