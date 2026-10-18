<script lang="ts">
    import { invoke } from '@tauri-apps/api/tauri';
    import { onMount } from 'svelte';
    import type { Watch } from '$types';
    
    let watches: Watch[] = [];
    let isLoading = true;
    let error: string | null = null;
    
    // Nouveau formulaire de veille
    let newWatch = {
        name: '',
        query: '',
        language: 'fr',
        sources: ['google_news'] as string[],
        frequency: 60,
        isActive: true,
    };
    
    // Options disponibles
    const languageOptions = [
        { code: 'fr', label: 'Français' },
        { code: 'en', label: 'Anglais' },
        { code: 'es', label: 'Espagnol' },
        { code: 'de', label: 'Allemand' },
        { code: 'ru', label: 'Russe' },
        { code: 'ar', label: 'Arabe' },
        { code: 'zh', label: 'Chinois' },
    ];
    
    const sourceOptions = [
        { id: 'google_news', label: 'Google Actualités', category: 'Moteurs de recherche' },
        { id: 'qwant', label: 'Qwant Actualités', category: 'Moteurs de recherche' },
        { id: 'yandex', label: 'Yandex Actualités', category: 'Moteurs de recherche' },
        { id: 'bing_news', label: 'Bing Actualités', category: 'Moteurs de recherche' },
        { id: 'twitter', label: 'Twitter/X', category: 'Réseaux sociaux' },
        { id: 'telegram', label: 'Telegram', category: 'Réseaux sociaux' },
        { id: 'discord', label: 'Discord', category: 'Réseaux sociaux' },
        { id: 'tiktok', label: 'TikTok', category: 'Réseaux sociaux' },
        { id: 'vkontakte', label: 'ВКонтакте', category: 'Réseaux sociaux' },
    ];
    
    const frequencyOptions = [
        { value: 5, label: '5 minutes' },
        { value: 15, label: '15 minutes' },
        { value: 30, label: '30 minutes' },
        { value: 60, label: '1 heure' },
        { value: 120, label: '2 heures' },
        { value: 240, label: '4 heures' },
        { value: 1440, label: '1 jour' },
    ];
    
    // État de modification
    let editingWatch: Watch | null = null;
    let isSubmitting = false;
    
    onMount(async () => {
        await loadWatches();
    });
    
    async function loadWatches() {
        isLoading = true;
        error = null;
        
        try {
            watches = await invoke('get_watches') as Watch[];
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur inconnue';
        } finally {
            isLoading = false;
        }
    }
    
    async function addWatch() {
        if (!newWatch.name || !newWatch.query) {
            error = 'Le nom et la requête sont obligatoires';
            return;
        }
        
        isSubmitting = true;
        error = null;
        
        try {
            await invoke('add_watch', { watch: newWatch });
            await loadWatches();
            
            // Réinitialiser le formulaire
            newWatch = {
                name: '',
                query: '',
                language: 'fr',
                sources: ['google_news'],
                frequency: 60,
                isActive: true,
            };
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors de l\'ajout de la veille';
        } finally {
            isSubmitting = false;
        }
    }
    
    async function updateWatch() {
        if (!editingWatch) return;
        
        isSubmitting = true;
        error = null;
        
        try {
            await invoke('update_watch', {
                id: editingWatch.id,
                ...editingWatch,
            });
            await loadWatches();
            editingWatch = null;
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors de la mise à jour';
        } finally {
            isSubmitting = false;
        }
    }
    
    async function deleteWatch(id: number) {
        if (!confirm('Êtes-vous sûr de vouloir supprimer cette veille ?')) return;
        
        isSubmitting = true;
        error = null;
        
        try {
            await invoke('delete_watch', { id });
            await loadWatches();
        } catch (err) {
            error = err instanceof Error ? err.message : 'Erreur lors de la suppression';
        } finally {
            isSubmitting = false;
        }
    }
    
    function startEditing(watch: Watch) {
        editingWatch = { ...watch };
    }
    
    function cancelEditing() {
        editingWatch = null;
    }
    
    function toggleWatchActive(watch: Watch) {
        const updatedWatch = { ...watch, isActive: !watch.isActive };
        invoke('update_watch', updatedWatch);
        loadWatches();
    }
    
    function toggleSource(sourceId: string) {
        if (newWatch.sources.includes(sourceId)) {
            newWatch.sources = newWatch.sources.filter(s => s !== sourceId);
        } else {
            newWatch.sources = [...newWatch.sources, sourceId];
        }
    }
    
    function toggleEditingSource(sourceId: string) {
        if (!editingWatch) return;
        
        if (editingWatch.sources.includes(sourceId)) {
            editingWatch.sources = editingWatch.sources.filter(s => s !== sourceId);
        } else {
            editingWatch.sources = [...editingWatch.sources, sourceId];
        }
        editingWatch = { ...editingWatch };
    }
</script>

<div class="space-y-6">
    <div>
        <h2 class="text-2xl font-bold">Configuration des veilles</h2>
        <p class="text-gray-400">Gérez les veilles automatiques sur différentes sources</p>
    </div>
    
    <!-- Erreur -->
    {#if error}
        <div class="bg-red-900/30 border border-red-600 rounded-lg p-4">
            <p class="text-red-400">{error}</p>
            <button on:click={() => error = null} class="mt-2 text-red-400 hover:text-red-300">
                Fermer
            </button>
        </div>
    {/if}
    
    <!-- Formulaire de nouvelle veille -->
    <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <h3 class="text-xl font-semibold mb-4">
            {editingWatch ? 'Modifier la veille' : 'Nouvelle veille'}
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
                <label class="block text-sm font-medium mb-2">Nom de la veille</label>
                <input 
                    type="text" 
                    bind:value={editingWatch ? editingWatch.name : newWatch.name}
                    placeholder="Ex: Veille Ukraine"
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                />
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Requête de recherche</label>
                <input 
                    type="text" 
                    bind:value={editingWatch ? editingWatch.query : newWatch.query}
                    placeholder="Ex: Guerre Ukraine 2024"
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                />
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Langue</label>
                <select 
                    bind:value={editingWatch ? editingWatch.language : newWatch.language}
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    {#each languageOptions as option}
                        <option value={option.code}>{option.label}</option>
                    {/each}
                </select>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Fréquence</label>
                <select 
                    bind:value={editingWatch ? editingWatch.frequency : newWatch.frequency}
                    class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                >
                    {#each frequencyOptions as option}
                        <option value={option.value}>{option.label}</option>
                    {/each}
                </select>
            </div>
            
            <!-- Sources -->
            <div class="md:col-span-2">
                <label class="block text-sm font-medium mb-2">Sources à surveiller</label>
                <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2">
                    {#each sourceOptions as source}
                        <label class="flex items-center space-x-2 p-2 bg-gray-700 rounded cursor-pointer hover:bg-gray-600 transition-colors">
                            <input 
                                type="checkbox" 
                                checked={editingWatch ? editingWatch.sources.includes(source.id) : newWatch.sources.includes(source.id)}
                                on:change={() => editingWatch ? toggleEditingSource(source.id) : toggleSource(source.id)}
                                class="h-4 w-4 text-blue-600"
                            />
                            <span class="text-sm">{source.label}</span>
                        </label>
                    {/each}
                </div>
            </div>
            
            <div class="md:col-span-2">
                <label class="flex items-center space-x-2 cursor-pointer">
                    <input 
                        type="checkbox" 
                        checked={editingWatch ? editingWatch.isActive : newWatch.isActive}
                        bind:checked={editingWatch ? editingWatch.isActive : newWatch.isActive}
                        class="h-4 w-4 text-blue-600"
                    />
                    <span class="text-sm font-medium">Veille active</span>
                </label>
            </div>
        </div>
        
        <div class="mt-6 flex space-x-4">
            {#if editingWatch}
                <button 
                    on:click={updateWatch}
                    disabled={isSubmitting || !editingWatch.name || !editingWatch.query}
                    class="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed px-4 py-2 rounded transition-colors font-medium"
                >
                    {isSubmitting ? 'Sauvegarde...' : 'Sauvegarder'}
                </button>
                <button 
                    on:click={cancelEditing}
                    class="bg-gray-600 hover:bg-gray-700 px-4 py-2 rounded transition-colors font-medium"
                >
                    Annuler
                </button>
            {:else}
                <button 
                    on:click={addWatch}
                    disabled={isSubmitting || !newWatch.name || !newWatch.query}
                    class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed px-4 py-2 rounded transition-colors font-medium"
                >
                    {isSubmitting ? 'Ajout...' : 'Ajouter la veille'}
                </button>
            {/if}
        </div>
    </div>
    
    <!-- Liste des veilles existantes -->
    <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <h3 class="text-xl font-semibold mb-4">Veilles existantes ({watches.length})</h3>
        
        {#if isLoading}
            <div class="flex items-center justify-center p-8">
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
                <p class="ml-4">Chargement...</p>
            </div>
        {:else if watches.length === 0}
            <p class="text-gray-400 text-center py-8">Aucune veille configurée</p>
        {:else}
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead>
                        <tr class="border-b border-gray-700">
                            <th class="text-left p-3">Nom</th>
                            <th class="text-left p-3">Requête</th>
                            <th class="text-left p-3">Sources</th>
                            <th class="text-left p-3">Langue</th>
                            <th class="text-left p-3">Fréquence</th>
                            <th class="text-left p-3">Statut</th>
                            <th class="text-right p-3">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each watches as watch}
                            <tr class="border-b border-gray-700 hover:bg-gray-700/50">
                                <td class="p-3">
                                    <span class="font-medium">{watch.name}</span>
                                </td>
                                <td class="p-3 truncate max-w-xs">
                                    {watch.query}
                                </td>
                                <td class="p-3">
                                    <div class="flex flex-wrap gap-1">
                                        {#each watch.sources.slice(0, 2) as source, i}
                                            <span class="bg-gray-700 px-2 py-1 rounded text-xs">{source}</span>
                                        {/each}
                                        {#if watch.sources.length > 2}
                                            <span class="bg-gray-700 px-2 py-1 rounded text-xs">+{watch.sources.length - 2}</span>
                                        {/if}
                                    </div>
                                </td>
                                <td class="p-3">
                                    {languageOptions.find(l => l.code === watch.language)?.label || watch.language}
                                </td>
                                <td class="p-3">
                                    {frequencyOptions.find(f => f.value === watch.frequency)?.label || `${watch.frequency} min`}
                                </td>
                                <td class="p-3">
                                    <span class:active={watch.isActive} class:inactive={!watch.isActive}>
                                        {watch.isActive ? 'Active' : 'Inactive'}
                                    </span>
                                </td>
                                <td class="p-3 text-right">
                                    <div class="flex justify-end space-x-2">
                                        <button 
                                            on:click={() => startEditing(watch)}
                                            class="text-blue-400 hover:text-blue-300"
                                            title="Modifier"
                                        >
                                            <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                                                <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
                                            </svg>
                                        </button>
                                        <button 
                                            on:click={() => toggleWatchActive(watch)}
                                            class="text-green-400 hover:text-green-300"
                                            title={watch.isActive ? 'Désactiver' : 'Activer'}
                                        >
                                            {#if watch.isActive}
                                                <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                    <circle cx="12" cy="12" r="10"/>
                                                    <line x1="12" y1="8" x2="12" y2="12"/>
                                                    <line x1="8" y1="12" x2="16" y2="12"/>
                                                </svg>
                                            {:else}
                                                <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                    <circle cx="12" cy="12" r="10"/>
                                                </svg>
                                            {/if}
                                        </button>
                                        <button 
                                            on:click={() => deleteWatch(watch.id)}
                                            class="text-red-400 hover:text-red-300"
                                            title="Supprimer"
                                        >
                                            <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                <polyline points="3 6 5 6 21 6"/>
                                                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                                            </svg>
                                        </button>
                                    </div>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </div>
</div>

<style>
    .active {
        color: #10b981;
    }
    .inactive {
        color: #6b7280;
    }
</style>
