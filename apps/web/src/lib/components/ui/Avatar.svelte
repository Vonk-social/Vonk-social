<script lang="ts">
	type Props = {
		url?: string | null;
		name: string;
		size?: number;
		/** Wrap with a gradient spark ring (for unseen stories). */
		ringed?: boolean;
		/** Dimmed ring once all stories are seen. */
		seen?: boolean;
	};
	let { url, name, size = 48, ringed = false, seen = false }: Props = $props();
	const initial = $derived(name.trim().charAt(0).toUpperCase() || '?');
</script>

{#if ringed}
	<span
		class="inline-flex items-center justify-center rounded-full p-[2px]"
		class:vonk-spark={!seen}
		class:bg-border={seen}
		style="width: {size + 4}px; height: {size + 4}px;"
	>
		<span class="flex items-center justify-center rounded-full bg-white p-[2px]"
			style="width: {size}px; height: {size}px;">
			{#if url}
				<img src={url} alt={name} width={size - 4} height={size - 4}
					class="rounded-full object-cover" style="width: {size - 4}px; height: {size - 4}px;" />
			{:else}
				<span class="flex items-center justify-center rounded-full bg-terracotta font-bold text-white"
					style="width: {size - 4}px; height: {size - 4}px; font-size: {Math.round((size - 4) * 0.42)}px;"
				>{initial}</span>
			{/if}
		</span>
	</span>
{:else if url}
	<img src={url} alt={name} width={size} height={size}
		class="rounded-full border border-border object-cover"
		style="width: {size}px; height: {size}px;" />
{:else}
	<span class="inline-flex items-center justify-center rounded-full bg-terracotta font-bold text-white"
		style="width: {size}px; height: {size}px; font-size: {Math.round(size * 0.42)}px;"
		aria-label={name}
	>{initial}</span>
{/if}
