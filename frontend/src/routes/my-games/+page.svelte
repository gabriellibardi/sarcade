<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { apiRequest, errorMessage } from '$lib/api';
	import { authState, loadAuth } from '$lib/auth';
	import Alert from '$lib/components/ui/Alert.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import type { OwnedGame, OwnedRevision } from '$lib/types/review';
	import { formatDate, playerCountLabel } from '$lib/utils/format';

	let games: OwnedGame[] = [];
	let isLoading = true;
	let error = '';

	const statusLabel: Record<OwnedRevision['status'], string> = {
		draft: 'Rascunho',
		pending: 'Em revisão',
		approved: 'Aprovada',
		rejected: 'Rejeitada',
		withdrawn: 'Retirada'
	};

	const statusClass: Record<OwnedRevision['status'], string> = {
		draft: 'bg-slate-200 text-slate-700',
		pending: 'bg-amber-950 text-amber-300',
		approved: 'bg-emerald-950 text-emerald-300',
		rejected: 'bg-red-950 text-red-300',
		withdrawn: 'bg-slate-800 text-slate-300'
	};

	async function loadGames() {
		isLoading = true;
		error = '';
		try {
			games = await apiRequest<OwnedGame[]>('/api/me/games');
		} catch (loadError) {
			error = errorMessage(loadError, 'Não foi possível carregar seus jogos.');
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
		await loadGames();
	}

	onMount(() => void initializePage());
</script>

<svelte:head>
	<title>Meus jogos — Sarcade</title>
	<meta name="description" content="Acompanhe as revisões dos seus jogos no Sarcade." />
</svelte:head>

<main class="page-shell max-w-5xl">
	<PageHeader
		eyebrow="Área do autor"
		title="Meus jogos"
		description="Acompanhe versões publicadas, revisões pendentes e as justificativas de revisões rejeitadas."
	/>

	<div class="mt-8">
		{#if isLoading}
			<div class="surface-card h-72 animate-pulse" aria-label="Carregando seus jogos"></div>
		{:else if error}
			<Alert tone="error" message={error} onRetry={loadGames} />
		{:else if games.length === 0}
			<EmptyState
				title="Você ainda não enviou jogos"
				description="Envie seu primeiro projeto para começar a acompanhar suas revisões."
			>
				<a href={resolve('/submit')} class="primary-button">Publicar um jogo</a>
			</EmptyState>
		{:else}
			<div class="space-y-6">
				{#each games as game (game.id)}
					<section class="surface-card overflow-hidden">
						<header
							class="flex flex-wrap items-center justify-between gap-3 border-b border-white/10 px-5 py-4"
						>
							<div>
								<p class="eyebrow">Jogo #{game.id}</p>
								<h2 class="display-title mt-1 text-2xl text-slate-950">
									{game.revisions[0]?.title ?? 'Jogo sem revisão'}
								</h2>
							</div>
							{#if game.archived_at}
								<span class="status-chip bg-slate-800 px-3 py-1 text-xs font-bold text-slate-300"
									>Arquivado</span
								>
							{/if}
						</header>

						<ul class="divide-y-2 divide-slate-200">
							{#each game.revisions as revision (revision.revision_id)}
								<li class="grid gap-3 px-5 py-4 sm:grid-cols-[1fr_auto] sm:items-center">
									<div>
										<div class="flex flex-wrap items-center gap-2">
											<strong class="text-slate-900">Versão {revision.revision_number}</strong>
											<span
												class={`status-chip px-2.5 py-1 text-xs font-bold ${statusClass[revision.status]}`}
											>
												{statusLabel[revision.status]}
											</span>
										</div>
										<p class="mt-1 text-sm text-slate-500">
											{revision.runtime_name} · {revision.runtime_version} · {playerCountLabel(
												revision.min_players,
												revision.max_players
											)}
											· criada em {formatDate(revision.created_at) ?? 'data não informada'}
										</p>
										{#if revision.status === 'rejected' && revision.review_message}
											<p class="mt-2 text-sm text-red-700">Motivo: {revision.review_message}</p>
										{/if}
									</div>
									<a
										href={resolve('/my-games/revisions/[id]', { id: String(revision.revision_id) })}
										class="secondary-button px-4 py-2 text-sm">Ver revisão</a
									>
								</li>
							{/each}
						</ul>
					</section>
				{/each}
			</div>
		{/if}
	</div>
</main>
