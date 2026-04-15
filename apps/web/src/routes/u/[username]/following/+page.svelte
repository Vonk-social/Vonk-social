<script lang="ts">
	import FollowList from '$lib/components/follow/FollowList.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import type { PageProps } from './$types';
	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>@{data.username} volgt — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/u/{data.username}" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">
		← @{data.username}
	</a>
	<h1 class="mb-4 font-display text-2xl font-bold text-ink">{data.title}</h1>
	<FollowList items={data.list.data} />
</main>

<Toast />
