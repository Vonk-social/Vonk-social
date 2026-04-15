<script lang="ts">
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import type { StoryGroup } from '$lib/api/feed';

	type Props = { groups: StoryGroup[]; onOpen?: (g: StoryGroup) => void };
	let { groups, onOpen }: Props = $props();
</script>

{#if groups.length > 0}
	<div class="-mx-4 mb-4 overflow-x-auto">
		<ul class="flex gap-4 px-4 py-2">
			{#each groups as g (g.author.uuid)}
				<li class="shrink-0 text-center">
					<button
						type="button"
						class="focus-visible:outline-2"
						aria-label="Stories van {g.author.display_name}, {g.unseen_count} ongezien"
						onclick={() => onOpen?.(g)}
					>
						<Avatar
							url={g.author.avatar_url}
							name={g.author.display_name}
							size={64}
							ringed
							seen={g.unseen_count === 0}
						/>
						<p class="mt-1 max-w-[80px] truncate text-xs text-ink">
							{g.author.display_name}
						</p>
					</button>
				</li>
			{/each}
		</ul>
	</div>
{/if}
