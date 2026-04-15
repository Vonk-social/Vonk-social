<script lang="ts">
	import type { MediaRef } from '$lib/api/posts';

	type Props = { media: MediaRef[]; onMediaClick?: (m: MediaRef, idx: number) => void };
	let { media, onMediaClick }: Props = $props();

	function src(m: MediaRef): string {
		return m.variants.medium ?? m.variants.full ?? m.variants.thumb ?? '';
	}

	// Tailwind class derived from count: 1→full, 2→2 cols, 3→2 cols +spanning, 4→2×2.
	const gridClass = $derived.by(() => {
		switch (media.length) {
			case 0:
				return '';
			case 1:
				return 'grid grid-cols-1';
			case 2:
				return 'grid grid-cols-2 gap-1';
			case 3:
			case 4:
				return 'grid grid-cols-2 gap-1';
			default:
				return 'grid grid-cols-2 gap-1';
		}
	});

	const aspect = $derived(media.length === 1 ? 'aspect-[4/5]' : 'aspect-square');
</script>

{#if media.length > 0}
	<div class="mt-3 overflow-hidden rounded-[var(--radius-lg)] {gridClass}">
		{#each media as m, i (m.uuid)}
			<button
				type="button"
				class="relative block {aspect} overflow-hidden bg-border focus-visible:outline-2"
				class:col-span-2={media.length === 3 && i === 0}
				onclick={() => onMediaClick?.(m, i)}
			>
				<img
					src={src(m)}
					alt={m.alt_text ?? ''}
					class="h-full w-full object-cover"
					loading="lazy"
					decoding="async"
				/>
			</button>
		{/each}
	</div>
{/if}
