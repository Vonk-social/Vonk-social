<script lang="ts">
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import type { FollowListItem } from '$lib/api/follows';

	type Props = { items: FollowListItem[] };
	let { items }: Props = $props();
</script>

{#if items.length === 0}
	<div class="vonk-card text-center text-muted">Niemand hier.</div>
{:else}
	<ul class="vonk-card divide-y divide-border p-0">
		{#each items as u (u.uuid)}
			<li class="flex items-center gap-3 p-4">
				<a href="/u/{u.username}" class="shrink-0">
					<Avatar url={u.avatar_url} name={u.display_name} size={48} />
				</a>
				<div class="min-w-0 flex-1">
					<a href="/u/{u.username}" class="block truncate font-bold text-ink hover:underline">
						{u.display_name}
					</a>
					<p class="truncate text-sm text-muted">
						@{u.username}{u.is_private ? ' · 🔒' : ''}
					</p>
				</div>
			</li>
		{/each}
	</ul>
{/if}
