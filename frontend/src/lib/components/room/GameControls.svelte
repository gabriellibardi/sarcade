<script lang="ts">
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';

	type Control = { input: string; label: string; name: string };

	let {
		enabled,
		onPadDown,
		onPadUp
	}: {
		enabled: boolean;
		onPadDown: (input: string) => void;
		onPadUp: (input: string) => void;
	} = $props();

	const controls: Control[] = [
		{ input: 'Up', label: '↑', name: 'Cima' },
		{ input: 'Left', label: '←', name: 'Esquerda' },
		{ input: 'Down', label: '↓', name: 'Baixo' },
		{ input: 'Right', label: '→', name: 'Direita' },
		{ input: 'ActionA', label: 'A', name: 'Ação A' },
		{ input: 'ActionB', label: 'B', name: 'Ação B' }
	];

	const directionalControls = controls.slice(0, 4);
	const actionControls = controls.slice(4).reverse();
	const directionPositions: Record<string, string> = {
		Up: 'col-start-2 row-start-1',
		Left: 'col-start-1 row-start-2',
		Down: 'col-start-2 row-start-3',
		Right: 'col-start-3 row-start-2'
	};

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

{#snippet padButton(control: Control, className: string)}
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
		class={className}
	>
		{control.label}
	</button>
{/snippet}

<section
	class="pointer-events-none absolute inset-x-3 bottom-3 z-10 flex items-end justify-between sm:hidden"
	aria-label="Controles do jogo"
>
	<div class="pointer-events-auto grid h-30 w-30 shrink-0 grid-cols-3 grid-rows-3">
		{#each directionalControls as control (control.input)}
			<div class={directionPositions[control.input]}>
				{@render padButton(
					control,
					'flex h-10 w-10 touch-none select-none items-center justify-center border-2 border-white/45 bg-black/60 text-lg font-black text-white shadow-[2px_2px_0_rgba(0,0,0,0.65)] backdrop-blur-sm transition active:scale-90 active:border-indigo-300 active:bg-arcade-blue/85 disabled:opacity-30'
				)}
			</div>
		{/each}
	</div>

	<div class="pointer-events-auto flex shrink-0 items-end gap-2 pb-4">
		{#each actionControls as control, index (control.input)}
			<div class={index === 0 ? 'translate-y-1' : '-translate-y-3'}>
				{@render padButton(
					control,
					'flex h-12 w-12 touch-none select-none items-center justify-center rounded-full border-2 border-white/50 bg-arcade-red/75 font-mono text-base font-black text-white shadow-[3px_3px_0_rgba(81,18,67,0.8)] backdrop-blur-sm transition active:scale-90 active:bg-[#c64a4a] active:shadow-none disabled:opacity-30'
				)}
			</div>
		{/each}
	</div>
</section>

<section class="surface-card mt-5 hidden p-6 sm:block">
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

	<div class="mt-5 grid grid-cols-6 gap-2">
		{#each controls as control (control.input)}
			{@render padButton(
				control,
				'flex min-h-14 touch-none select-none items-center justify-center border-2 border-slate-300 bg-slate-50 text-lg font-black text-slate-800 shadow-[3px_3px_0_#3d1a62] transition hover:translate-x-0.5 hover:translate-y-0.5 hover:border-indigo-300 hover:bg-indigo-50 hover:text-indigo-300 hover:shadow-[1px_1px_0_#3d1a62] active:translate-x-1 active:translate-y-1 active:shadow-none disabled:cursor-not-allowed disabled:opacity-40 disabled:shadow-none'
			)}
		{/each}
	</div>
</section>
