<script lang="ts">
	import { resolve } from '$app/paths';
	import type { GameSummary } from '$lib/types/game';
	import { playerCountLabel } from '$lib/utils/format';

	let { game, variant = 'default' }: { game: GameSummary; variant?: 'default' | 'compact' } =
		$props();

	const playerLabel = $derived(playerCountLabel(game.min_players, game.max_players));
	const isCompact = $derived(variant === 'compact');
</script>

<article
	class="group relative flex h-full flex-col overflow-hidden rounded-xl border border-white/10 bg-[#0d1016]/92 shadow-[0_18px_60px_rgb(0_0_0/0.3)] backdrop-blur-md transition duration-200 focus-within:-translate-y-1 focus-within:border-arcade-red/70 hover:-translate-y-1 hover:border-arcade-red/70 hover:shadow-[0_22px_70px_rgb(0_0_0/0.38)]"
>
	<a
		href={resolve('/games/[id]', { id: String(game.id) })}
		class="absolute inset-0 z-10"
		aria-label={`Abrir detalhes de ${game.title}`}
	></a>

	<div
		class={`relative overflow-hidden bg-[#11141b] ${isCompact ? 'aspect-[4/3]' : 'aspect-[16/9]'}`}
	>
		{#if game.has_thumbnail}
			<img
				src={`/api/games/${game.id}/thumbnail`}
				alt={`Capa do jogo ${game.title}`}
				class="h-full w-full object-cover opacity-90 transition duration-500 group-hover:scale-[1.035] group-hover:opacity-100"
			/>
		{:else}
			<div
				class="flex h-full items-center justify-center bg-[linear-gradient(145deg,#171b31,#28121f)]"
				aria-label="Jogo sem imagem de capa"
			>
				<span
					class="display-title rounded-lg border border-white/15 bg-arcade-bg/70 px-6 py-3 text-5xl text-arcade-ink"
				>
					{game.title.slice(0, 1).toUpperCase()}
				</span>
			</div>
		{/if}

		{#if !isCompact}
			<div class="absolute inset-x-3 top-3 flex items-start justify-between gap-2">
				<span
					class="rounded-full border border-indigo-300/50 bg-arcade-bg/80 px-2.5 py-1 text-[10px] font-semibold text-indigo-300 backdrop-blur-md"
				>
					{game.runtime_name}
				</span>
				{#if game.is_featured}
					<span
						class="rounded-full border border-arcade-gold/60 bg-[#261d10]/85 px-2.5 py-1 text-[10px] font-semibold text-arcade-gold backdrop-blur-md"
					>
						Destaque
					</span>
				{/if}
			</div>
		{/if}
	</div>

	<div class={`flex flex-1 flex-col ${isCompact ? 'p-3.5' : 'p-5'}`}>
		<div class={`flex items-start justify-between gap-3 ${isCompact ? 'mb-1.5' : 'mb-3'}`}>
			<h3
				class={`${isCompact ? 'text-base' : 'text-xl'} font-semibold tracking-tight text-slate-950 transition group-hover:text-white`}
			>
				{game.title}
			</h3>
			{#if !isCompact}
				<span
					class="status-chip shrink-0 bg-slate-100/80 px-2 py-1 text-[10px] font-semibold text-slate-600"
				>
					{playerLabel}
				</span>
			{/if}
		</div>

		{#if !isCompact}
			<p class="line-clamp-3 flex-1 text-sm leading-6 text-slate-600">{game.description}</p>
		{/if}

		<div
			class={`flex items-center justify-between gap-3 ${isCompact ? 'mt-auto pt-1' : 'mt-5 border-t border-white/8 pt-4'}`}
		>
			<div class={`min-w-0 text-slate-500 ${isCompact ? 'text-[10px]' : 'text-[11px] leading-5'}`}>
				{#if game.author_id && game.author_name}
					<p class="truncate">
						{isCompact ? '' : 'por '}<a
							href={resolve('/users/[id]', { id: String(game.author_id) })}
							class="relative z-20 font-medium text-slate-700 hover:text-indigo-300 hover:underline"
							>{game.author_name}</a
						>
					</p>
				{:else}
					<p class="truncate">autor não informado</p>
				{/if}
				{#if !isCompact}<p class="truncate">{game.runtime_version}</p>{/if}
			</div>

			{#if isCompact}
				<span
					class="status-chip shrink-0 border-arcade-red bg-arcade-red px-2 py-0.5 text-[9px] font-medium text-white"
				>
					{playerLabel}
				</span>
			{:else}
				<span class="primary-button shrink-0 px-4 py-2 text-xs" aria-hidden="true">Jogar</span>
			{/if}
		</div>
	</div>
</article>
