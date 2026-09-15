<script lang="ts">
	let { events }: { events: string[] } = $props();
</script>

<details class="surface-card overflow-hidden">
	<summary class="cursor-pointer px-6 py-4 text-sm font-bold text-slate-700 hover:bg-slate-50"
		>Diagnóstico da conexão</summary
	>
	<div class="border-t border-slate-200 p-5">
		{#if events.length === 0}
			<p class="text-sm text-slate-500">Nenhum evento recebido.</p>
		{:else}
			<pre
				class="max-h-80 overflow-auto rounded-xl bg-slate-950 p-4 text-xs leading-5 text-slate-300">{events.join(
					'\n\n'
				)}</pre>
		{/if}
	</div>
</details>
