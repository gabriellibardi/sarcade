<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { apiRequest, errorMessage } from '$lib/api';
	import { authState, loadAuth } from '$lib/auth';
	import Alert from '$lib/components/ui/Alert.svelte';
	import type { ReviewDetails } from '$lib/types/review';
	import { formatDate, playerCountLabel } from '$lib/utils/format';

	let revision: ReviewDetails | null = null;
	let isLoading = true;
	let error = '';
	let actionMessage = '';
	let actionError = '';
	let isActing = false;
	const revisionId = Number(page.params.id);

	async function loadRevision() {
		isLoading = true;
		error = '';
		try {
			revision = await apiRequest<ReviewDetails>(`/api/revisions/${revisionId}`);
		} catch (loadError) {
			error = errorMessage(loadError, 'Não foi possível carregar esta revisão.');
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	async function initializePage() {
		await loadAuth();
		if (!get(authState).user) {
			await goto(resolve('/submit'));
			return;
		}
		await loadRevision();
	}

	async function editRevision() {
		if (!revision || isActing) return;
		isActing = true;
		actionError = '';
		try {
			const editable = await apiRequest<{ revision_id: number }>(
				`/api/revisions/${revision.revision_id}/edit`,
				{ method: 'POST' }
			);
			await goto(resolve('/my-games/revisions/[id]/edit', { id: String(editable.revision_id) }));
		} catch (actionFailure) {
			actionError = errorMessage(actionFailure, 'Não foi possível preparar a revisão para edição.');
			console.error(actionFailure);
		} finally {
			isActing = false;
		}
	}

	async function submitDraft() {
		if (!revision || isActing) return;
		isActing = true;
		actionError = '';
		actionMessage = '';
		try {
			await apiRequest(`/api/revisions/${revision.revision_id}/submit`, { method: 'POST' });
			actionMessage = 'Revisão enviada e aguardando análise.';
			await loadRevision();
		} catch (actionFailure) {
			actionError = errorMessage(actionFailure, 'Não foi possível enviar a revisão.');
			console.error(actionFailure);
		} finally {
			isActing = false;
		}
	}

	async function testRevision() {
		if (!revision || isActing) return;
		isActing = true;
		actionError = '';
		try {
			const result = await apiRequest<{
				code: string;
				host_token: string;
				invite_token: string;
			}>(`/api/revisions/${revision.revision_id}/rooms`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ nickname: get(authState).user?.name.slice(0, 30) || 'Autor' })
			});
			await goto(
				resolve(
					`/rooms/[code]?nickname=${encodeURIComponent(get(authState).user?.name.slice(0, 30) || 'Autor')}&host_token=${encodeURIComponent(result.host_token)}&invite_token=${encodeURIComponent(result.invite_token)}`,
					{ code: result.code }
				)
			);
		} catch (actionFailure) {
			actionError = errorMessage(actionFailure, 'Não foi possível criar a sala de teste.');
			console.error(actionFailure);
		} finally {
			isActing = false;
		}
	}

	onMount(() => void initializePage());
</script>

<svelte:head>
	<title
		>{revision
			? `${revision.title} — revisão ${revision.revision_number}`
			: 'Revisão — Sarcade'}</title
	>
</svelte:head>

<main class="page-shell max-w-5xl">
	<a
		href={resolve('/my-games')}
		class="font-mono text-xs font-bold text-indigo-300 uppercase hover:underline"
	>
		← Voltar para meus jogos
	</a>

	{#if isLoading}
		<div class="surface-card mt-7 h-96 animate-pulse" aria-label="Carregando revisão"></div>
	{:else if error}
		<div class="mt-7"><Alert tone="error" message={error} onRetry={loadRevision} /></div>
	{:else if revision}
		<header class="mt-7">
			<p class="eyebrow">Jogo #{revision.id} · versão {revision.revision_number}</p>
			<h1 class="display-title mt-3 text-4xl text-slate-950">{revision.title}</h1>
			<p class="mt-3 text-slate-600">
				{playerCountLabel(revision.min_players, revision.max_players)} · {revision.runtime_name}
				{revision.runtime_version}
			</p>
		</header>

		<div class="mt-8 grid items-start gap-6 lg:grid-cols-[1fr_18rem]">
			<div class="space-y-6">
				<section class="surface-card p-6">
					<h2 class="text-xl font-black text-slate-950">Descrição</h2>
					<p class="mt-3 leading-7 whitespace-pre-line text-slate-600">{revision.description}</p>
				</section>
				<section class="surface-card overflow-hidden">
					<div class="border-b-2 border-slate-200 px-6 py-4">
						<h2 class="text-xl font-black text-slate-950">Código-fonte</h2>
					</div>
					<pre
						class="max-h-[42rem] overflow-auto bg-[#0d0d0d] p-6 text-sm leading-6 text-slate-300"><code
							>{revision.source_code}</code
						></pre>
				</section>
			</div>

			<aside class="surface-card p-5 lg:sticky lg:top-24">
				<h2 class="font-black text-slate-950">Revisão</h2>
				<dl class="mt-4 space-y-3 text-sm">
					<div>
						<dt class="text-slate-500">Estado</dt>
						<dd class="font-bold text-slate-800">{revision.status}</dd>
					</div>
					<div>
						<dt class="text-slate-500">Criada em</dt>
						<dd class="font-bold text-slate-800">
							{formatDate(revision.created_at) ?? 'Não informado'}
						</dd>
					</div>
					{#if revision.reviewed_at}
						<div>
							<dt class="text-slate-500">Revisada em</dt>
							<dd class="font-bold text-slate-800">
								{formatDate(revision.reviewed_at) ?? 'Não informado'}
							</dd>
						</div>
					{/if}
				</dl>
				{#if revision.review_message}
					<div class="mt-5">
						<Alert tone="info" title="Justificativa" message={revision.review_message} />
					</div>
				{/if}
				<div class="mt-5 grid gap-2 border-t-2 border-slate-200 pt-5">
					<button
						type="button"
						onclick={() => void testRevision()}
						disabled={isActing}
						class="primary-button"
					>
						{isActing ? 'Preparando...' : 'Testar multiplayer'}
					</button>
					{#if revision.status === 'draft'}
						<a
							href={resolve('/my-games/revisions/[id]/edit', { id: String(revision.revision_id) })}
							class="secondary-button text-center">Editar rascunho</a
						>
						<button
							type="button"
							onclick={() => void submitDraft()}
							disabled={isActing}
							class="primary-button"
						>
							{isActing ? 'Enviando...' : 'Enviar para revisão'}
						</button>
					{:else}
						<button
							type="button"
							onclick={() => void editRevision()}
							disabled={isActing}
							class="secondary-button"
						>
							{isActing
								? 'Preparando...'
								: revision.status === 'pending'
									? 'Retirar e editar'
									: 'Criar nova versão'}
						</button>
					{/if}
				</div>
				{#if actionMessage}<div class="mt-4">
						<Alert tone="success" message={actionMessage} />
					</div>{/if}
				{#if actionError}<div class="mt-4"><Alert tone="error" message={actionError} /></div>{/if}
			</aside>
		</div>
	{/if}
</main>
