<script lang="ts">
	type Tone = 'info' | 'success' | 'warning' | 'error';

	let {
		message,
		title,
		tone = 'info',
		onRetry,
		dismissAfter,
		onDismiss
	}: {
		message: string;
		title?: string;
		tone?: Tone;
		onRetry?: () => void;
		dismissAfter?: number;
		onDismiss?: () => void;
	} = $props();

	let visible = $state(true);

	$effect(() => {
		if (!message) {
			visible = false;
			return;
		}

		visible = true;

		if (!dismissAfter) return;

		const timeout = window.setTimeout(() => {
			visible = false;
			onDismiss?.();
		}, dismissAfter);

		return () => window.clearTimeout(timeout);
	});

	const toneClasses: Record<Tone, string> = {
		info: 'border-indigo-300 bg-[#1c2035] text-indigo-900 shadow-[4px_4px_0_#3d1a62]',
		success: 'border-emerald-600 bg-[#14281f] text-emerald-100 shadow-[4px_4px_0_#143b2c]',
		warning: 'border-amber-500 bg-[#2b2112] text-amber-100 shadow-[4px_4px_0_#5b3c13]',
		error: 'border-red-500 bg-[#2d181a] text-red-100 shadow-[4px_4px_0_#511243]'
	};
</script>

{#if visible}
	<div class={`border-2 p-5 ${toneClasses[tone]}`} role={tone === 'error' ? 'alert' : 'status'}>
		{#if title}
			<p class="font-bold">{title}</p>
		{/if}
		<p class={title ? 'mt-1 text-sm leading-6' : 'text-sm leading-6'}>{message}</p>
		{#if onRetry}
			<button type="button" onclick={onRetry} class="mt-3 text-sm font-bold underline"
				>Tentar novamente</button
			>
		{/if}
	</div>
{/if}
