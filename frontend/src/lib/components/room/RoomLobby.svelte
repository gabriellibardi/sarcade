<script lang="ts">
	import { resolve } from '$app/paths';
	import type { RoomSnapshot } from '$lib/types/room';

	let {
		room,
		myPlayerId,
		amHost,
		canStart,
		onStart,
		onKick,
		onPromote
	}: {
		room: RoomSnapshot;
		myPlayerId: number | null;
		amHost: boolean;
		canStart: boolean;
		onStart: () => void;
		onKick: (playerId: number) => void;
		onPromote: (playerId: number) => void;
	} = $props();
</script>

<section class="surface-card overflow-hidden">
	<div
		class="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 px-6 py-5"
	>
		<div>
			<p class="text-xs font-bold tracking-widest text-slate-400 uppercase">Lobby</p>
			<h2 class="mt-1 text-xl font-black text-slate-950">Participantes</h2>
		</div>
		<span
			class={`rounded-full px-3 py-1 text-xs font-bold ${room.status === 'waiting' ? 'bg-amber-100 text-amber-800' : 'bg-emerald-100 text-emerald-800'}`}
		>
			{room.status === 'waiting' ? 'Aguardando' : 'Em partida'}
		</span>
	</div>

	<div class="grid gap-6 p-6 lg:grid-cols-[1fr_15rem]">
		<div>
			<ul class="space-y-2">
				{#each room.players as player (player.id)}
					<li
						class="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-slate-200 bg-slate-50/70 p-3"
					>
						<div class="flex min-w-0 items-center gap-3">
							<span
								class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-white text-sm font-black text-indigo-700 shadow-sm"
								>{player.id}</span
							>
							<div class="min-w-0">
								<p class="truncate font-bold text-slate-900">{player.nickname}</p>
								<p class="text-xs text-slate-500">
									{player.id === myPlayerId ? 'Você' : 'Participante'}{player.is_host
										? ' · host'
										: ''}
								</p>
							</div>
						</div>

						{#if amHost && player.id !== myPlayerId}
							<div class="flex gap-1">
								<button
									type="button"
									onclick={() => onPromote(player.id)}
									class="ghost-button px-2.5 py-1.5 text-xs">Tornar host</button
								>
								<button
									type="button"
									onclick={() => onKick(player.id)}
									class="ghost-button px-2.5 py-1.5 text-xs text-red-700 hover:bg-red-50"
									>Remover</button
								>
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		</div>

		<aside class="rounded-2xl bg-slate-950 p-5 text-sm text-slate-300">
			<p class="text-xs font-bold tracking-widest text-indigo-300 uppercase">Partida</p>
			<dl class="mt-4 space-y-3">
				<div>
					<dt class="text-slate-500">Jogo</dt>
					<dd>
						<a
							href={resolve('/games/[id]', { id: String(room.game_id) })}
							class="font-bold text-white hover:underline">Jogo #{room.game_id}</a
						>
					</dd>
				</div>
				<div>
					<dt class="text-slate-500">Ocupação</dt>
					<dd class="font-bold text-white">{room.players.length} de {room.max_players}</dd>
				</div>
				<div>
					<dt class="text-slate-500">Mínimo</dt>
					<dd class="font-bold text-white">{room.min_players} participantes</dd>
				</div>
			</dl>

			<button
				type="button"
				onclick={onStart}
				disabled={!canStart}
				class="primary-button mt-5 w-full px-3 py-2 text-sm"
			>
				Iniciar partida
			</button>
			{#if !amHost}
				<p class="mt-2 text-xs leading-5 text-slate-400">Somente o host pode iniciar.</p>
			{:else if room.players.length < room.min_players}
				<p class="mt-2 text-xs leading-5 text-slate-400">Aguardando mais participantes.</p>
			{/if}
		</aside>
	</div>
</section>
