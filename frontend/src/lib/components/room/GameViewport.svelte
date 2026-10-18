<script lang="ts">
	let {
		frameUrl,
		sinteract,
		canvas = $bindable(null)
	}: {
		frameUrl: string;
		sinteract: boolean;
		canvas?: HTMLCanvasElement | null;
	} = $props();
</script>

<div class="overflow-hidden bg-[#0d0d0d] shadow-[0_0_0_2px_#24283b]">
	<div class="flex min-h-64 items-center justify-center">
		{#if sinteract}
			<canvas
				bind:this={canvas}
				tabindex="0"
				aria-label="Tela interativa da partida"
				class="block aspect-video max-h-[calc(100dvh-16rem)] min-h-64 w-full touch-none bg-white object-contain outline-none focus:ring-2 focus:ring-indigo-300"
			></canvas>
		{:else if frameUrl}
			<img
				src={frameUrl}
				alt="Tela atual da partida"
				class="block max-h-[calc(100dvh-16rem)] min-h-64 w-full bg-white object-contain"
			/>
		{:else}
			<div
				class="flex min-h-72 items-center justify-center border-2 border-dashed border-slate-700 text-center font-mono text-xs text-slate-400 uppercase"
			>
				O runtime ainda não enviou um quadro.
			</div>
		{/if}
	</div>
</div>
