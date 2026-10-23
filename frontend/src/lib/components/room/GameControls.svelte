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
	class="mt-4 flex items-center justify-between gap-6 px-2 sm:mx-auto sm:max-w-3xl sm:px-6"
	aria-label="Controles do jogo"
>
	<div class="grid h-30 w-30 shrink-0 grid-cols-3 grid-rows-3 sm:h-36 sm:w-36">
		{#each directionalControls as control (control.input)}
			<div class={directionPositions[control.input]}>
				{@render padButton(
					control,
					'flex h-10 w-10 touch-none select-none items-center justify-center border-2 border-slate-400 bg-slate-800 text-lg font-black text-white shadow-[3px_3px_0_#3d1a62] transition active:translate-x-0.5 active:translate-y-0.5 active:border-indigo-300 active:bg-arcade-blue active:shadow-none disabled:opacity-30 sm:h-12 sm:w-12'
				)}
			</div>
		{/each}
	</div>

	<div class="flex shrink-0 items-end gap-3 pb-4 sm:gap-5">
		{#each actionControls as control, index (control.input)}
			<div class={index === 0 ? 'translate-y-1' : '-translate-y-3'}>
				{@render padButton(
					control,
					'flex h-12 w-12 touch-none select-none items-center justify-center rounded-full border-2 border-red-300 bg-arcade-red font-mono text-base font-black text-white shadow-[3px_3px_0_#511243] transition active:translate-x-0.5 active:translate-y-0.5 active:bg-[#c64a4a] active:shadow-none disabled:opacity-30 sm:h-14 sm:w-14'
				)}
			</div>
		{/each}
	</div>
</section>
