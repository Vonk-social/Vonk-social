<script lang="ts">
	import { untrack } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import FollowButton from '$lib/components/follow/FollowButton.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import { searchUsers, type UserCard } from '$lib/api/users';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let q = $state('');
	let results = $state<UserCard[]>([]);
	let searching = $state(false);
	let debounceId: ReturnType<typeof setTimeout> | undefined;

	const suggestions = untrack(() => data.suggestions);

	$effect(() => {
		const term = q.trim();
		clearTimeout(debounceId);
		if (term.length < 2) {
			results = [];
			return;
		}
		searching = true;
		debounceId = setTimeout(async () => {
			try {
				results = await searchUsers(term);
			} finally {
				searching = false;
			}
		}, 250);
	});
</script>

<svelte:head>
	<title>Zoek — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Feed</a>
	<h1 class="mb-4 font-display text-2xl font-bold text-ink">Zoek & ontdek</h1>

	<div class="mb-6">
		<input
			type="search"
			bind:value={q}
			placeholder="Zoek op naam of @gebruikersnaam…"
			autocomplete="off"
			class="w-full rounded-xl border border-border bg-white px-4 py-3 text-ink placeholder:text-muted focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
		/>
	</div>

	{#if q.trim().length >= 2}
		<section>
			<h2 class="mb-3 font-display text-lg font-bold text-ink">
				Resultaten {#if searching}<span class="text-sm text-muted">· laden…</span>{/if}
			</h2>
			{#if results.length === 0 && !searching}
				<div class="vonk-card text-center text-muted">Niemand gevonden.</div>
			{:else}
				<ul class="vonk-card divide-y divide-border p-0">
					{#each results as u (u.uuid)}
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
								{#if u.bio}
									<p class="mt-1 line-clamp-1 text-sm text-ink">{u.bio}</p>
								{/if}
							</div>
							<FollowButton username={u.username} initial={u.follow_state} />
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{:else}
		<section>
			<h2 class="mb-3 font-display text-lg font-bold text-ink">Misschien ken je</h2>
			{#if suggestions.length === 0}
				<div class="vonk-card text-muted">
					Nog niemand om voor te stellen. Probeer hierboven op naam te zoeken.
				</div>
			{:else}
				<ul class="vonk-card divide-y divide-border p-0">
					{#each suggestions as u (u.uuid)}
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
								{#if u.bio}
									<p class="mt-1 line-clamp-1 text-sm text-ink">{u.bio}</p>
								{/if}
							</div>
							<FollowButton username={u.username} initial={u.follow_state} />
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}
</main>

<Toast />
