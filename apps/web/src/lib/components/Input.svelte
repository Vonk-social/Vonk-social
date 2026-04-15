<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	type Props = {
		value: string;
		label: string;
		name: string;
		id?: string;
		placeholder?: string;
		autocomplete?: HTMLInputAttributes['autocomplete'];
		help?: string;
		error?: string;
		success?: string;
		disabled?: boolean;
	};

	let {
		value = $bindable(''),
		label,
		name,
		id = name,
		placeholder,
		autocomplete,
		help,
		error,
		success,
		disabled = false
	}: Props = $props();
</script>

<label class="block text-sm font-medium text-ink" for={id}>
	{label}
	<input
		{id}
		{name}
		{placeholder}
		{autocomplete}
		{disabled}
		bind:value
		class="mt-2 block w-full rounded-xl border border-border bg-white px-4 py-3 text-ink
		       placeholder:text-muted focus:border-terracotta focus:outline-none focus:ring-2
		       focus:ring-terracotta/40 disabled:bg-border/20"
		aria-invalid={error ? 'true' : undefined}
		aria-describedby={help || error || success ? `${id}-desc` : undefined}
	/>
	{#if error}
		<span id="{id}-desc" class="mt-1 block text-sm text-terracotta">{error}</span>
	{:else if success}
		<span id="{id}-desc" class="mt-1 block text-sm text-sage">{success}</span>
	{:else if help}
		<span id="{id}-desc" class="mt-1 block text-sm text-muted">{help}</span>
	{/if}
</label>
