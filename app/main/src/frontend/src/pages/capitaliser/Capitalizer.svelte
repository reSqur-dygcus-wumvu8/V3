<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/tauri';
    import { formatDate, getReliabilityLabel, getCredibilityLabel } from '$lib/utils';
    import type { ProcessedWatchResult, EntityReference } from '$types';
    
    // État
    let isLoading = true;
    let error: string | null = null;
    let results: ProcessedWatchResult[] = [];
    let selectedResult: ProcessedWatchResult | null = null;
    let entityForm = {
        type: 'individual' as 'individual' | 'organization' | 'event' | 'location' | 'object',
        name: '',
        description: '',
        additionalFields: {} as Record<string, string>,
    };
    
    // Options
    const entityTypes = [
        { value: 'individual', label: 'Individu' },
        { value: 'organization', label: 'Organisation' },
        { value: 'event', label: 'Événement' },
        { value: 'location', label: 'Lieu' },
        { value: 'object', label: 'Objet' },
    ];
    
    const individualTypes = [
        { value: 'real', label: 'Identité réelle' },
        { value: 'fictive', label: 'Identité fictive' },
        { value: 'digital', label: 'Identité numérique' },
    ];
    
    const locationTypes = [
        { value: 'real', label: 'Lieu réel' },
        { value: 'fictive', label: 'Lieu fictif' },
        { value: 'digital', label: 'Lieu numérique' },
    ];
    
    onMount(async () => {
        await loadResults();
    });
    
    async function loadResults() {
        isLoading = true;
        error = null;
        
        try {
            results = await invoke('get_processed_watch_results') as ProcessedWatchResult[];
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors du chargement des résultats';
        } finally {
            isLoading = false;
        }
    }
    
    function selectResult(result: ProcessedWatchResult) {
        selectedResult = result;
        
        // Pré-remplir le formulaire d'entité avec les informations du résultat
        entityForm = {
            type: 'individual',
            name: result.title,
            description: result.content.substring(0, 500), // Tronquer à 500 caractères
            additionalFields: {},
        };
    }
    
    function closeModal() {
        selectedResult = null;
        entityForm = {
            type: 'individual',
            name: '',
            description: '',
            additionalFields: {},
        };
    }
    
    async function createEntity() {
        if (!entityForm.name.trim()) {
            error = 'Le nom de l\'entité est obligatoire';
            return;
        }
        
        try {
            // Construire les données de l'entité en fonction du type
            const entityData: Record<string, any> = {
                name: entityForm.name,
                comments: entityForm.description,
            };
            
            // Ajouter les champs spécifiques selon le type
            if (entityForm.type === 'individual') {
                entityData.type = entityForm.additionalFields.individualType || 'real';
                entityData.firstName = entityForm.additionalFields.firstName || '';
                entityData.lastName = entityForm.additionalFields.lastName || '';
                entityData.biography = entityForm.description;
            } else if (entityForm.type === 'organization') {
                entityData.type = entityForm.additionalFields.organizationType || '';
                entityData.summary = entityForm.description;
            } else if (entityForm.type === 'event') {
                entityData.startDate = entityForm.additionalFields.startDate || '';
                entityData.endDate = entityForm.additionalFields.endDate || '';
                entityData.summary = entityForm.description;
            } else if (entityForm.type === 'location') {
                entityData.type = entityForm.additionalFields.locationType || 'real';
                entityData.address = entityForm.additionalFields.address || '';
                entityData.gpsLat = entityForm.additionalFields.gpsLat ? parseFloat(entityForm.additionalFields.gpsLat) : null;
                entityData.gpsLon = entityForm.additionalFields.gpsLon ? parseFloat(entityForm.additionalFields.gpsLon) : null;
                entityData.ipAddress = entityForm.additionalFields.ipAddress || '';
                entityData.summary = entityForm.description;
            } else if (entityForm.type === 'object') {
                entityData.type = entityForm.additionalFields.objectType || '';
                entityData.details = entityForm.description;
            }
            
            // Ajouter les entités détectées comme sources liées
            if (selectedResult) {
                entityData.sources = selectedResult.entities.map(e => ({
                    name: e.name,
                    url: selectedResult?.url || '',
                    reliability: selectedResult.source_rating,
                }));
            }
            
            // Appeler la fonction de création d'entité
            await invoke('create_entity', {
                entityType: entityForm.type,
                entityData,
            });
            
            // Fermer la modale et recharger les résultats
            closeModal();
            await loadResults();
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors de la création de l\'entité';
        }
    }
    
    function updateAdditionalField(key: string, value: string) {
        entityForm.additionalFields[key] = value;
        entityForm = { ...entityForm };
    }
    
    async function capitalizeWithCotation(result: ProcessedWatchResult) {
        try {
            // Capitaliser avec la cotation proposée
            await invoke('capitalize_info', {
                watchResultId: result.id,
                entityType: 'event',
                entityId: 0,
                cotation: result.proposed_rating,
            });
            
            await loadResults();
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors de la capitalisation';
        }
    }
</script>

<div class="space-y-6">
    <div class="flex items-center justify-between">
        <div>
            <h2 class="text-2xl font-bold">Capitalisation</h2>
            <p class="text-gray-400">Transformez les résultats de veille en connaissances structurées</p>
        </div>
        
        <div class="flex space-x-2">
            <button 
                on:click={loadResults}
                class="bg-gray-700 hover:bg-gray-600 px-3 py-2 rounded transition-colors text-sm"
            >
                <svg class="w-4 h-4 inline mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="23 4 23 10 17 10"/>
                    <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                </svg>
                Rafraîchir
            </button>
        </div>
    </div>
    
    <!-- Erreur -->
    {#if error}
        <div class="bg-red-900/30 border border-red-600 rounded-lg p-4">
            <p class="text-red-400">{error}</p>
            <button on:click={() => error = null} class="mt-2 text-red-400 hover:text-red-300 text-sm">
                Fermer
            </button>
        </div>
    {/if}
    
    <!-- Résultats à capitaliser -->
    {#if isLoading}
        <div class="flex items-center justify-center p-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            <p class="ml-4">Chargement des résultats à capitaliser...</p>
        </div>
    {:else if results.length === 0}
        <div class="bg-gray-800 rounded-lg p-8 border border-gray-700 text-center">
            <svg class="w-12 h-12 mx-auto mb-4 text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
                <line x1="16" y1="13" x2="8" y2="13"/>
                <line x1="16" y1="17" x2="8" y2="17"/>
            </svg>
            <p class="text-gray-400">Aucun résultat à capitaliser</p>
            <p class="text-gray-500 text-sm mt-2">Exécutez une veille ou traitez les résultats existants</p>
        </div>
    {:else}
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <h3 class="text-lg font-semibold mb-4">Résultats prêts à être capitalisés ({results.length})</h3>
            
            <div class="space-y-3">
                {#each results as result}
                    <div class="bg-gray-700 rounded p-3 border border-gray-600 hover:bg-gray-600/50 transition-colors">
                        <div class="flex items-start justify-between">
                            <div class="flex-1">
                                <div class="flex items-center space-x-2 mb-1">
                                    <h4 class="font-medium">{result.title}</h4>
                                    <span 
                                        class="text-xs px-2 py-1 rounded bg-blue-600 text-white"
                                        title="Cotation: {result.proposed_rating} - {getReliabilityLabel(result.source_rating)} / {getCredibilityLabel(result.info_rating)}"
                                    >
                                        {result.proposed_rating}
                                    </span>
                                </div>
                                
                                <p class="text-sm text-gray-400 mb-1">
                                    Source: {result.source_name} | {formatDate(result.published_at || result.fetched_at || '')}
                                </p>
                                
                                {#if result.entities && result.entities.length > 0}
                                    <div class="flex flex-wrap gap-1 mb-1">
                                        {#each result.entities.slice(0, 3) as entity, i}
                                            <span 
                                                class="bg-gray-600 px-2 py-1 rounded text-xs {entity.matched ? 'border border-green-500' : ''}"
                                                title="{entity.entity_type}: {entity.name}"
                                            >
                                                {entity.name}
                                            </span>
                                        {/each}
                                        {#if result.entities.length > 3}
                                            <span class="bg-gray-600 px-2 py-1 rounded text-xs">
                                                +{result.entities.length - 3} autres
                                            </span>
                                        {/if}
                                    </div>
                                {/if}
                            </div>
                            
                            <div class="flex space-x-2">
                                <button 
                                    on:click={() => selectResult(result)}
                                    class="bg-gray-600 hover:bg-gray-500 px-3 py-1 rounded transition-colors text-sm"
                                >
                                    Créer une fiche
                                </button>
                                
                                <button 
                                    on:click={() => capitalizeWithCotation(result)}
                                    class="bg-green-600 hover:bg-green-700 px-3 py-1 rounded transition-colors text-sm"
                                >
                                    Capitaliser
                                </button>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        </div>
    {/if}
</div>

<!-- Modale : Création d'une fiche d'entité -->
{#if selectedResult}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
        <div class="bg-gray-900 rounded-lg w-full max-w-2xl max-h-[90vh] overflow-y-auto p-6">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-xl font-bold">Créer une fiche d'entité</h2>
                <button on:click={closeModal} class="text-gray-400 hover:text-white">
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="18" y1="6" x2="6" y2="18"/>
                        <line x1="6" y1="6" x2="18" y2="18"/>
                    </svg>
                </button>
            </div>
            
            <div class="space-y-4">
                <!-- Type d'entité -->
                <div>
                    <label class="block text-sm font-medium mb-2">Type d'entité</label>
                    <div class="grid grid-cols-2 md:grid-cols-5 gap-2">
                        {#each entityTypes as type}
                            <label class="flex items-center space-x-2 p-2 bg-gray-800 rounded cursor-pointer hover:bg-gray-700 transition-colors">
                                <input 
                                    type="radio" 
                                    name="entityType" 
                                    value={type.value} 
                                    bind:group={entityForm.type}
                                    class="h-4 w-4 text-blue-600"
                                />
                                <span class="text-sm">{type.label}</span>
                            </label>
                        {/each}
                    </div>
                </div>
                
                <!-- Nom -->
                <div>
                    <label class="block text-sm font-medium mb-2">Nom *</label>
                    <input 
                        type="text" 
                        bind:value={entityForm.name}
                        placeholder="Nom de l'entité"
                        class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                    />
                </div>
                
                <!-- Description -->
                <div>
                    <label class="block text-sm font-medium mb-2">Description</label>
                    <textarea 
                        bind:value={entityForm.description}
                        placeholder="Description ou résumé"
                        class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none h-32 resize-none"
                    ></textarea>
                </div>
                
                <!-- Champs spécifiques selon le type -->
                {#if entityForm.type === 'individual'}
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium mb-2">Type d'individu</label>
                            <div class="flex space-x-2">
                                {#each individualTypes as type}
                                    <label class="flex items-center space-x-1 p-2 bg-gray-800 rounded cursor-pointer hover:bg-gray-700 transition-colors">
                                        <input 
                                            type="radio" 
                                            name="individualType" 
                                            value={type.value} 
                                            on:change={(e) => updateAdditionalField('individualType', e.target.value)}
                                            class="h-4 w-4 text-blue-600"
                                        />
                                        <span class="text-sm">{type.label}</span>
                                    </label>
                                {/each}
                            </div>
                        </div>
                        
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm font-medium mb-2">Prénom</label>
                                <input 
                                    type="text" 
                                    on:input={(e) => updateAdditionalField('firstName', e.target.value)}
                                    placeholder="Prénom"
                                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium mb-2">Nom</label>
                                <input 
                                    type="text" 
                                    on:input={(e) => updateAdditionalField('lastName', e.target.value)}
                                    placeholder="Nom"
                                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                />
                            </div>
                        </div>
                    </div>
                {:else if entityForm.type === 'organization'}
                    <div>
                        <label class="block text-sm font-medium mb-2">Type d'organisation</label>
                        <input 
                            type="text" 
                            on:input={(e) => updateAdditionalField('organizationType', e.target.value)}
                            placeholder="Ex: Gouvernement, Entreprise, ONG"
                            class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                        />
                    </div>
                {:else if entityForm.type === 'event'}
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium mb-2">Date de début</label>
                            <input 
                                type="datetime-local" 
                                on:input={(e) => updateAdditionalField('startDate', e.target.value)}
                                class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium mb-2">Date de fin</label>
                            <input 
                                type="datetime-local" 
                                on:input={(e) => updateAdditionalField('endDate', e.target.value)}
                                class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                            />
                        </div>
                    </div>
                {:else if entityForm.type === 'location'}
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium mb-2">Type de lieu</label>
                            <div class="flex space-x-2">
                                {#each locationTypes as type}
                                    <label class="flex items-center space-x-1 p-2 bg-gray-800 rounded cursor-pointer hover:bg-gray-700 transition-colors">
                                        <input 
                                            type="radio" 
                                            name="locationType" 
                                            value={type.value} 
                                            on:change={(e) => updateAdditionalField('locationType', e.target.value)}
                                            class="h-4 w-4 text-blue-600"
                                        />
                                        <span class="text-sm">{type.label}</span>
                                    </label>
                                {/each}
                            </div>
                        </div>
                        
                        <div>
                            <label class="block text-sm font-medium mb-2">Adresse</label>
                            <input 
                                type="text" 
                                on:input={(e) => updateAdditionalField('address', e.target.value)}
                                placeholder="Adresse postale"
                                class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                            />
                        </div>
                        
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm font-medium mb-2">Latitude</label>
                                <input 
                                    type="number" 
                                    step="0.000001" 
                                    on:input={(e) => updateAdditionalField('gpsLat', e.target.value)}
                                    placeholder="Ex: 48.8566"
                                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium mb-2">Longitude</label>
                                <input 
                                    type="number" 
                                    step="0.000001" 
                                    on:input={(e) => updateAdditionalField('gpsLon', e.target.value)}
                                    placeholder="Ex: 2.3522"
                                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                />
                            </div>
                        </div>
                        
                        <div>
                            <label class="block text-sm font-medium mb-2">Adresse IP (pour les lieux numériques)</label>
                            <input 
                                type="text" 
                                on:input={(e) => updateAdditionalField('ipAddress', e.target.value)}
                                placeholder="Ex: 192.168.1.1"
                                class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                            />
                        </div>
                    </div>
                {:else if entityForm.type === 'object'}
                    <div>
                        <label class="block text-sm font-medium mb-2">Type d'objet</label>
                        <input 
                            type="text" 
                            on:input={(e) => updateAdditionalField('objectType', e.target.value)}
                            placeholder="Ex: Véhicule, Arme, Document"
                            class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                        />
                    </div>
                {/if}
                
                <!-- Sources liées -->
                <div>
                    <label class="block text-sm font-medium mb-2">Sources liées</label>
                    {#if selectedResult && selectedResult.entities && selectedResult.entities.length > 0}
                        <div class="flex flex-wrap gap-2">
                            {#each selectedResult.entities as entity}
                                <span class="bg-gray-700 px-2 py-1 rounded text-sm">
                                    {entity.name} ({entity.entity_type})
                                </span>
                            {/each}
                        </div>
                        <p class="text-xs text-gray-500 mt-2">
                            Ces sources seront automatiquement liées à l'entité
                        </p>
                    {:else}
                        <p class="text-gray-400 text-sm">Aucune source liée</p>
                    {/if}
                </div>
            </div>
            
            <div class="flex justify-end space-x-2 mt-6">
                <button 
                    on:click={closeModal}
                    class="bg-gray-600 hover:bg-gray-500 px-4 py-2 rounded transition-colors"
                >
                    Annuler
                </button>
                
                <button 
                    on:click={createEntity}
                    disabled={!entityForm.name.trim()}
                    class="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed px-4 py-2 rounded transition-colors"
                >
                    Créer l'entité
                </button>
            </div>
        </div>
    </div>
{/if}
</div>

<style>
    /* Style pour les boutons radio personnalisés */
    input[type="radio"] {
        accent-color: #3b82f6;
    }
</style>
