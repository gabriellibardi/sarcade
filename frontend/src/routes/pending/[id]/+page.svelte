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

	let game: ReviewDetails | null = null;
	let isLoading = true;
	let error = '';
	let reviewMessage = '';
	let actionMessage = '';
	let actionError = '';
	let isReviewing = false;

	const revisionId = Number(page.params.id);

	async function loadRevision() {
		isLoading = true;
		error = '';

		try {
			game = await apiRequest<ReviewDetails>(`/api/revisions/${revisionId}`);
		} catch (loadError) {
			error = errorMessage(loadError, 'Não foi possível carregar esta revisão.');
			console.error(loadError);
		} finally {
			isLoading = false;
		}
	}

	async function approveRevision() {
		if (!game || isReviewing) return;

		isReviewing = true;
		actionMessage = '';
		actionError = '';

		try {
			await apiRequest(`/api/revisions/${game.revision_id}/approve`, { method: 'POST' });
			actionMessage = 'Revisão aprovada e jogo publicado no catálogo.';
			await loadRevision();
		} catch (reviewError) {
			actionError = errorMessage(reviewError, 'Não foi possível aprovar a revisão.');
			console.error(reviewError);
		} finally {
			isReviewing = false;
		}
	}

	async function rejectRevision() {
		if (!game || isReviewing) return;

		reviewMessage = reviewMessage.trim();
		if (!reviewMessage) {
			actionError = 'Informe uma justificativa para a rejeição.';
			return;
		}

		isReviewing = true;
		actionMessage = '';
		actionError = '';

		try {
			await apiRequest(`/api/revisions/${game.revision_id}/reject`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ message: reviewMessage })
			});
			actionMessage = 'Revisão rejeitada e justificativa registrada.';
			reviewMessage = '';
			await loadRevision();
		} catch (reviewError) {
			actionError = errorMessage(reviewError, 'Não foi possível rejeitar a revisão.');
			console.error(reviewError);
		} finally {
			isReviewing = false;
		}
	}

	async function editRevision() {
		if (!game || isReviewing) return;
		isReviewing = true;
		actionError = '';
		try {
			const editable = await apiRequest<{ revision_id: number }>(
				`/api/revisions/${game.revision_id}/edit`,
				{ method: 'POST' }
			);
			await goto(resolve('/my-games/revisions/[id]/edit', { id: String(editable.revision_id) }));
		} catch (editError) {
			actionError = errorMessage(editError, 'Não foi possível preparar uma nova versão.');
			console.error(editError);
		} finally {
			isReviewing = false;
		}
	}

	async function testRevision() {
		if (!game || isReviewing) return;
		isReviewing = true;
		actionError = '';
		try {
			const nickname = get(authState).user?.name.slice(0, 30) || 'Revisor';
			const result = await apiRequest<{
				code: string;
				host_token: string;
				invite_token: string;
			}>(`/api/revisions/${game.revision_id}/rooms`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ nickname })
			});
			await goto(
				resolve(
					`/rooms/[code]?nickname=${encodeURIComponent(nickname)}&host_token=${encodeURIComponent(result.host_token)}&invite_token=${encodeURIComponent(result.invite_token)}`,
					{ code: result.code }
				)
			);
		} catch (testError) {
			actionError = errorMessage(testError, 'Não foi possível criar a sala de teste.');
			console.error(testError);
		} finally {
			isReviewing = false;
		}
	}

	async function initializePage() {
		await loadAuth();

		if (!get(authState).user?.is_admin) {
			await goto(resolve('/'));
			return;
		}

		await loadRevision();
	}

	onMount(() => {
		void initializePage();
	});
</script>

<svelte:head>
	<title>{game ? `Revisar ${game.title} — Sarcade` : 'Revisão — Sarcade'}</title>
</svelte:head>

<main class="page-shell max-w-5xl">
	<a
		href={resolve('/pending')}
		class="font-mono text-xs font-bold text-indigo-300 uppercase hover:underline"
		>← Voltar para a fila</a
	>

	{#if isLoading}
		<div class="surface-card mt-7 h-96 animate-pulse" aria-label="Carregando revisão"></div>
	{:else if error}
		<div class="mt-7"><Alert tone="error" message={error} onRetry={loadRevision} /></div>
	{:else if game}
		<header class="mt-7 flex flex-col gap-5 sm:flex-row sm:items-start sm:justify-between">
			<div class="max-w-3xl">
				<p class="eyebrow">Revisão #{game.revision_id}</p>
				<h1 class="display-title mt-3 text-4xl text-slate-950">{game.title}</h1>
				<p class="mt-3 text-slate-600">
					Jogo #{game.id} · enviado por {game.author_name ?? 'autor não informado'}
				</p>
			</div>
			<span
				class={`status-chip px-3 py-1.5 text-sm font-bold ${game.status === 'pending' ? 'bg-amber-950 text-amber-300' : game.status === 'approved' ? 'bg-emerald-950 text-emerald-300' : 'bg-red-950 text-red-300'}`}
			>
				{game.status === 'pending'
					? 'Pendente'
					: game.status === 'approved'
						? 'Aprovado'
						: 'Rejeitado'}
			</span>
		</header>

		<div class="mt-8 grid items-start gap-6 lg:grid-cols-[1fr_18rem]">
			<div class="space-y-6">
				<section class="surface-card overflow-hidden">
					{#if game.has_thumbnail}
						<img
							src={`/api/revisions/${game.revision_id}/thumbnail`}
							alt={`Capa de ${game.title}`}
							class="max-h-80 w-full object-cover"
						/>
					{/if}
					<div class="p-6">
						<h2 class="text-xl font-black text-slate-950">Descrição</h2>
						<p class="mt-3 leading-7 whitespace-pre-line text-slate-600">{game.description}</p>
					</div>
				</section>

				<section class="surface-card overflow-hidden">
					<div
						class="flex flex-wrap items-center justify-between gap-3 border-b-2 border-slate-200 px-6 py-4"
					>
						<h2 class="text-xl font-black text-slate-950">Código-fonte</h2>
						<code class="rounded-lg bg-slate-100 px-2.5 py-1 text-xs font-bold text-slate-600"
							>{game.source_extension}</code
						>
					</div>
					<pre
						class="max-h-[38rem] overflow-auto bg-[#0d0d0d] p-6 text-sm leading-6 text-slate-300"><code
							>{game.source_code}</code
						></pre>
				</section>
			</div>

			<aside class="space-y-5 lg:sticky lg:top-24">
				<section class="surface-card p-5">
					<h2 class="font-black text-slate-950">Metadados</h2>
					<dl class="mt-4 space-y-3 text-sm">
						<div>
							<dt class="text-slate-500">Linguagem e versão</dt>
							<dd class="font-bold text-slate-800">{game.runtime_name} · {game.runtime_version}</dd>
						</div>
						<div>
							<dt class="text-slate-500">Identificador da versão</dt>
							<dd class="font-mono text-xs break-all text-slate-700">{game.runtime_build_id}</dd>
						</div>
						<div>
							<dt class="text-slate-500">Jogadores</dt>
							<dd class="font-bold text-slate-800">
								{playerCountLabel(game.min_players, game.max_players)}
							</dd>
						</div>
						<div>
							<dt class="text-slate-500">Enviado em</dt>
							<dd class="font-bold text-slate-800">
								{formatDate(game.created_at) ?? 'Data não informada'}
							</dd>
						</div>
						{#if game.reviewed_at}
							<div>
								<dt class="text-slate-500">Revisado em</dt>
								<dd class="font-bold text-slate-800">
									{formatDate(game.reviewed_at) ?? 'Data não informada'}
								</dd>
							</div>
						{/if}
					</dl>
				</section>

				<section class="surface-card p-5">
					<h2 class="font-black text-slate-950">Decisão</h2>
					<button
						type="button"
						onclick={() => void testRevision()}
						disabled={isReviewing}
						class="secondary-button mt-4 w-full"
					>
						Testar jogo
					</button>
					{#if game.status !== 'pending'}
						<p class="mt-3 text-sm leading-6 text-slate-600">Esta revisão já foi analisada.</p>
						{#if game.review_message}
							<div class="mt-4">
								<Alert tone="info" title="Justificativa" message={game.review_message} />
							</div>
						{/if}
						<button
							type="button"
							onclick={() => void editRevision()}
							disabled={isReviewing}
							class="secondary-button mt-4 w-full"
						>
							Criar versão editável
						</button>
					{:else}
						<label for="reviewMessage" class="form-label mt-4">Justificativa da rejeição</label>
						<textarea
							id="reviewMessage"
							bind:value={reviewMessage}
							rows="4"
							class="form-control"
							placeholder="Obrigatória para rejeitar"
						></textarea>
						<div class="mt-4 grid gap-2">
							<button
								type="button"
								onclick={() => void approveRevision()}
								disabled={isReviewing}
								class="primary-button border-emerald-500 bg-emerald-700 shadow-[4px_4px_0_#143b2c] hover:bg-emerald-600"
							>
								{isReviewing ? 'Processando...' : 'Aprovar e publicar'}
							</button>
							<button
								type="button"
								onclick={() => void rejectRevision()}
								disabled={isReviewing}
								class="secondary-button border-red-500 bg-[#34191c] text-red-200 shadow-[4px_4px_0_#511243] hover:border-red-400 hover:bg-[#432025]"
							>
								Rejeitar
							</button>
						</div>
					{/if}
				</section>

				{#if actionMessage}<Alert tone="success" message={actionMessage} />{/if}
				{#if actionError}<Alert tone="error" message={actionError} />{/if}
			</aside>
		</div>
	{/if}
</main>
