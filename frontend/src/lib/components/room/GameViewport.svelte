<script lang="ts">
	let { canvas = $bindable(null) }: { canvas?: HTMLCanvasElement | null } = $props();
</script>

<div class="overflow-hidden bg-black shadow-[0_0_0_2px_#24283b]">
	<div class="flex min-h-64 items-center justify-center bg-black">
		<canvas
			bind:this={canvas}
			tabindex="0"
			aria-label="Tela interativa da partida"
			class="block aspect-video max-h-[calc(100dvh-16rem)] min-h-64 w-full touch-none bg-black object-contain outline-none focus:ring-2 focus:ring-indigo-300"
		></canvas>
	</div>
</div>
