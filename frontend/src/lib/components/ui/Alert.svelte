<script lang="ts">
	type Tone = 'info' | 'success' | 'warning' | 'error';

	let {
		message,
		title,
		tone = 'info',
		onRetry
	}: { message: string; title?: string; tone?: Tone; onRetry?: () => void } = $props();

	const toneClasses: Record<Tone, string> = {
		info: 'border-indigo-200 bg-indigo-50 text-indigo-900',
		success: 'border-emerald-200 bg-emerald-50 text-emerald-900',
		warning: 'border-amber-200 bg-amber-50 text-amber-900',
		error: 'border-red-200 bg-red-50 text-red-900'
	};
</script>

<div
	class={`rounded-2xl border p-5 ${toneClasses[tone]}`}
	role={tone === 'error' ? 'alert' : 'status'}
>
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
