<script lang="ts">
	let { enabled, onPress }: { enabled: boolean; onPress: (input: string) => void } = $props();

	const controls = [
		{ input: 'Up', label: '↑', name: 'Cima' },
		{ input: 'Left', label: '←', name: 'Esquerda' },
		{ input: 'Down', label: '↓', name: 'Baixo' },
		{ input: 'Right', label: '→', name: 'Direita' },
		{ input: 'ActionA', label: 'A', name: 'Ação A' },
		{ input: 'ActionB', label: 'B', name: 'Ação B' }
	];
</script>

<section class="surface-card p-6">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<p class="text-xs font-bold tracking-widest text-slate-400 uppercase">Entrada</p>
			<h2 class="mt-1 text-xl font-black text-slate-950">Controles</h2>
		</div>
		<span
			class={`rounded-full px-3 py-1 text-xs font-bold ${enabled ? 'bg-emerald-100 text-emerald-800' : 'bg-slate-100 text-slate-600'}`}
		>
			{enabled ? 'Ativos' : 'Aguardando partida'}
		</span>
	</div>

	<div class="mt-5 grid grid-cols-3 gap-2 sm:grid-cols-6">
		{#each controls as control (control.input)}
			<button
				type="button"
				onclick={() => onPress(control.input)}
				disabled={!enabled}
				aria-label={control.name}
				class="flex min-h-14 items-center justify-center rounded-2xl border border-slate-200 bg-slate-50 text-lg font-black text-slate-800 transition hover:border-indigo-300 hover:bg-indigo-50 hover:text-indigo-700 disabled:cursor-not-allowed disabled:opacity-40"
			>
				{control.label}
			</button>
		{/each}
	</div>
</section>
