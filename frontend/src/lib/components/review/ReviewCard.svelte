<script lang="ts">
	import { resolve } from '$app/paths';
	import type { GameSummary } from '$lib/types/game';
	import { formatDate, playerCountLabel } from '$lib/utils/format';

	let { game }: { game: GameSummary } = $props();
</script>

<article class="surface-card overflow-hidden p-5 sm:flex sm:gap-5">
	<div
		class="h-32 w-full shrink-0 overflow-hidden rounded-2xl bg-gradient-to-br from-indigo-100 via-sky-50 to-amber-100 sm:w-44"
	>
		{#if game.has_thumbnail}
			<img
				src={`/api/games/${game.id}/thumbnail`}
				alt={`Capa de ${game.title}`}
				class="h-full w-full object-cover"
			/>
		{:else}
			<div class="flex h-full items-center justify-center text-3xl font-black text-indigo-300">
				{game.title.slice(0, 1).toUpperCase()}
			</div>
		{/if}
	</div>

	<div class="mt-4 min-w-0 flex-1 sm:mt-0">
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div>
				<p class="text-xs font-bold tracking-widest text-amber-700 uppercase">
					Revisão #{game.revision_id}
				</p>
				<h2 class="mt-1 text-xl font-black text-slate-950">{game.title}</h2>
			</div>
			<span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-bold text-slate-700">
				{game.runtime_name} · {game.runtime_version}
			</span>
		</div>

		<p class="mt-2 line-clamp-2 text-sm leading-6 text-slate-600">{game.description}</p>
		<div class="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-slate-500">
			<span>por {game.author_name ?? 'autor não informado'}</span>
			<span>{playerCountLabel(game.min_players, game.max_players)}</span>
			<span>{formatDate(game.created_at, { dateStyle: 'short' }) ?? 'data não informada'}</span>
		</div>

		<a
			href={resolve('/pending/[id]', { id: String(game.revision_id) })}
			class="secondary-button mt-4 px-4 py-2 text-sm"
		>
			Analisar revisão
		</a>
	</div>
</article>
