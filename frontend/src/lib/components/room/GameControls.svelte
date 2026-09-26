<script lang="ts">
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';

	let {
		enabled,
		onPadDown,
		onPadUp
	}: {
		enabled: boolean;
		onPadDown: (input: string) => void;
		onPadUp: (input: string) => void;
	} = $props();

	const controls = [
		{ input: 'Up', label: '↑', name: 'Cima' },
		{ input: 'Left', label: '←', name: 'Esquerda' },
		{ input: 'Down', label: '↓', name: 'Baixo' },
		{ input: 'Right', label: '→', name: 'Direita' },
		{ input: 'ActionA', label: 'A', name: 'Ação A' },
		{ input: 'ActionB', label: 'B', name: 'Ação B' }
	];

	const activePointers = new SvelteMap<number, string>();
	const activeKeyboardInputs = new SvelteSet<string>();

	function handlePointerDown(event: PointerEvent, input: string) {
		if (!enabled || event.button !== 0 || activePointers.has(event.pointerId)) return;

		const button = event.currentTarget as HTMLButtonElement;
		button.setPointerCapture(event.pointerId);
		activePointers.set(event.pointerId, input);
		onPadDown(input);
	}

	function handlePointerEnd(event: PointerEvent) {
		const input = activePointers.get(event.pointerId);
		if (input === undefined) return;

		activePointers.delete(event.pointerId);
		onPadUp(input);
	}

	function handleKeyDown(event: KeyboardEvent, input: string) {
		if (
			!enabled ||
			(event.key !== ' ' && event.key !== 'Enter') ||
			event.repeat ||
			activeKeyboardInputs.has(input)
		) {
			return;
		}

		event.preventDefault();
		activeKeyboardInputs.add(input);
		onPadDown(input);
	}

	function releaseKeyboardInput(input: string) {
		if (!activeKeyboardInputs.delete(input)) return;
		onPadUp(input);
	}

	function handleKeyUp(event: KeyboardEvent, input: string) {
		if (event.key !== ' ' && event.key !== 'Enter') return;

		event.preventDefault();
		releaseKeyboardInput(input);
	}

	$effect(() => {
		if (enabled) return;

		for (const input of activePointers.values()) onPadUp(input);
		for (const input of activeKeyboardInputs) onPadUp(input);
		activePointers.clear();
		activeKeyboardInputs.clear();
	});
</script>

<section class="surface-card p-6">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<p class="text-xs font-bold tracking-widest text-slate-400 uppercase">Entrada</p>
			<h2 class="display-title mt-2 text-xl text-slate-950">Controles</h2>
		</div>
		<span
			class={`status-chip px-3 py-1 text-xs font-bold ${enabled ? 'bg-emerald-950 text-emerald-300' : 'bg-slate-100 text-slate-600'}`}
		>
			{enabled ? 'Ativos' : 'Aguardando partida'}
		</span>
	</div>

	<div class="mt-5 grid grid-cols-3 gap-2 sm:grid-cols-6">
		{#each controls as control (control.input)}
			<button
				type="button"
				onpointerdown={(event) => handlePointerDown(event, control.input)}
				onpointerup={handlePointerEnd}
				onpointercancel={handlePointerEnd}
				onlostpointercapture={handlePointerEnd}
				onkeydown={(event) => handleKeyDown(event, control.input)}
				onkeyup={(event) => handleKeyUp(event, control.input)}
				onblur={() => releaseKeyboardInput(control.input)}
				disabled={!enabled}
				aria-label={control.name}
				class="flex min-h-14 touch-none items-center justify-center border-2 border-slate-300 bg-slate-50 text-lg font-black text-slate-800 shadow-[3px_3px_0_#3d1a62] transition select-none hover:translate-x-0.5 hover:translate-y-0.5 hover:border-indigo-300 hover:bg-indigo-50 hover:text-indigo-300 hover:shadow-[1px_1px_0_#3d1a62] active:translate-x-1 active:translate-y-1 active:shadow-none disabled:cursor-not-allowed disabled:opacity-40 disabled:shadow-none"
			>
				{control.label}
			</button>
		{/each}
	</div>
</section>
