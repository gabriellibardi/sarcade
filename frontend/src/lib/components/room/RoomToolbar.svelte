<script lang="ts">
	let {
		code,
		isConnected,
		onCopy,
		onLeave
	}: { code: string; isConnected: boolean; onCopy: () => void; onLeave: () => void } = $props();
</script>

<div class="flex flex-wrap items-center justify-between gap-3">
	<div>
		<p class="eyebrow">Sala de jogo</p>
		<h1 class="display-title mt-2 text-3xl text-slate-950 sm:text-4xl">Sala {code}</h1>
	</div>
	<div class="flex flex-wrap gap-2">
		<button type="button" onclick={onCopy} class="secondary-button px-4 py-2 text-sm"
			>Copiar convite</button
		>
		{#if isConnected}
			<button
				type="button"
				onclick={onLeave}
				class="ghost-button px-4 py-2 text-sm text-red-400 hover:bg-red-950/30 hover:text-red-300"
				>Sair da sala</button
			>
		{/if}
	</div>
</div>
