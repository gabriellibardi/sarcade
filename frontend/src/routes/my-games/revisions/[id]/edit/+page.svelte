<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { apiRequest, errorMessage } from '$lib/api';
	import FilePicker from '$lib/components/forms/FilePicker.svelte';
	import Alert from '$lib/components/ui/Alert.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import type { ReviewDetails } from '$lib/types/review';
	import type { RuntimeBuild } from '$lib/types/runtime';

	let revision: ReviewDetails | null = null;
	let runtimes: RuntimeBuild[] = [];
	let title = '';
	let description = '';
	let sourceCode = '';
	let runtimeBuildId = '';
	let thumbnailFiles: FileList | null = null;
	let isLoading = true;
	let isSaving = false;
	let error = '';
	let saveError = '';
	const revisionId = Number(page.params.id);

	$: selectedRuntime = runtimes.find((runtime) => runtime.id === runtimeBuildId) ?? null;

	async function loadPage() {
		isLoading = true;
		error = '';
		try {
			const [loadedRevision, loadedRuntimes] = await Promise.all([
				apiRequest<ReviewDetails>(`/api/revisions/${revisionId}`),
				apiRequest<RuntimeBuild[]>('/api/runtimes')
			]);
			if (loadedRevision.status !== 'draft') {
				await goto(resolve('/my-games/revisions/[id]', { id: String(revisionId) }));
				return;
			}
			revision = loadedRevision;
			runtimes = loadedRuntimes;
			title = loadedRevision.title;
			description = loadedRevision.description;
			sourceCode = loadedRevision.source_code;
			runtimeBuildId = loadedRevision.runtime_build_id;
		} catch (loadError) {
			error = errorMessage(loadError, 'Não foi possível carregar o rascunho.');
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	async function saveDraft() {
		if (!revision || !selectedRuntime || isSaving) return;
		isSaving = true;
		saveError = '';
		const formData = new FormData();
		formData.append('title', title.trim());
		formData.append('description', description.trim());
		formData.append('runtime_build_id', selectedRuntime.id);
		formData.append(
			'source_code',
			new File([sourceCode], `game${selectedRuntime.source_extension}`, { type: 'text/plain' })
		);
		if (thumbnailFiles?.length) formData.append('thumbnail', thumbnailFiles[0]);

		try {
			await apiRequest(`/api/revisions/${revision.revision_id}`, {
				method: 'PATCH',
				body: formData
			});
			await goto(resolve('/my-games/revisions/[id]', { id: String(revision.revision_id) }));
		} catch (saveFailure) {
			saveError = errorMessage(saveFailure, 'Não foi possível salvar o rascunho.');
			console.error(saveFailure);
		} finally {
			isSaving = false;
		}
	}

	onMount(() => void loadPage());
</script>

<svelte:head><title>Editar revisão — Sarcade</title></svelte:head>

<main class="page-shell max-w-5xl">
	<PageHeader
		eyebrow="Rascunho"
		title={revision ? `Editar ${revision.title}` : 'Editar revisão'}
		description="As alterações serão analisadas novamente e os limites de participantes serão extraídos do código."
	/>

	{#if isLoading}
		<div class="surface-card mt-8 h-96 animate-pulse" aria-label="Carregando rascunho"></div>
	{:else if error}
		<div class="mt-8"><Alert tone="error" message={error} onRetry={loadPage} /></div>
	{:else if revision}
		<form
			onsubmit={(event) => {
				event.preventDefault();
				void saveDraft();
			}}
			class="surface-card mt-8 space-y-6 p-6 sm:p-8"
		>
			<div>
				<label for="title" class="form-label">Título</label>
				<input id="title" bind:value={title} required maxlength="100" class="form-control" />
			</div>
			<div>
				<label for="description" class="form-label">Descrição</label>
				<textarea
					id="description"
					bind:value={description}
					required
					maxlength="2000"
					rows="5"
					class="form-control"
				></textarea>
			</div>
			<div>
				<label for="runtime" class="form-label">Linguagem e versão</label>
				<select id="runtime" bind:value={runtimeBuildId} required class="form-control">
					{#each runtimes as runtime (runtime.id)}
						<option value={runtime.id}>{runtime.display_name} — {runtime.display_version}</option>
					{/each}
				</select>
			</div>
			<div>
				<label for="source" class="form-label">Código-fonte</label>
				<textarea
					id="source"
					bind:value={sourceCode}
					required
					rows="24"
					spellcheck="false"
					class="form-control font-mono text-sm"
				></textarea>
			</div>
			<FilePicker
				id="thumbnail"
				label="Substituir capa (opcional)"
				accept="image/png,image/jpeg,image/webp"
				bind:files={thumbnailFiles}
				buttonLabel="Escolher nova imagem"
			/>
			{#if saveError}<Alert tone="error" message={saveError} />{/if}
			<div class="flex flex-wrap gap-3">
				<button type="submit" disabled={isSaving || !selectedRuntime} class="primary-button">
					{isSaving ? 'Analisando e salvando...' : 'Salvar rascunho'}
				</button>
				<a
					href={resolve('/my-games/revisions/[id]', { id: String(revision.revision_id) })}
					class="ghost-button">Cancelar</a
				>
			</div>
		</form>
	{/if}
</main>
