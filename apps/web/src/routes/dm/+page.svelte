<script lang="ts">
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import SnapViewer from '$lib/components/snaps/SnapViewer.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let tab = $state<'inbox' | 'sent'>('inbox');
	let viewingUuid = $state<string | null>(null);

	function relTime(iso: string): string {
		const diff = Math.max(0, Math.round((Date.now() - new Date(iso).getTime()) / 1000));
		if (diff < 60) return 'zojuist';
		if (diff < 3600) return `${Math.round(diff / 60)} min`;
		if (diff < 86400) return `${Math.round(diff / 3600)} u`;
		return `${Math.round(diff / 86400)} d`;
	}
</script>

<svelte:head>
	<title>Snaps — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Feed</a>

	<h1 class="mb-4 font-display text-2xl font-bold text-ink">Snaps</h1>

	<div class="mb-4 flex gap-2 border-b border-border">
		<button
			type="button"
			class="px-4 py-2 text-sm font-semibold transition-colors"
			class:border-b-2={tab === 'inbox'}
			class:border-terracotta={tab === 'inbox'}
			class:text-ink={tab === 'inbox'}
			class:text-muted={tab !== 'inbox'}
			onclick={() => (tab = 'inbox')}
		>
			Inbox ({data.inbox.length})
		</button>
		<button
			type="button"
			class="px-4 py-2 text-sm font-semibold transition-colors"
			class:border-b-2={tab === 'sent'}
			class:border-terracotta={tab === 'sent'}
			class:text-ink={tab === 'sent'}
			class:text-muted={tab !== 'sent'}
			onclick={() => (tab = 'sent')}
		>
			Verzonden ({data.sent.length})
		</button>
	</div>

	{#if tab === 'inbox'}
		{#if data.inbox.length === 0}
			<div class="vonk-card text-center text-muted">
				Nog geen snaps. <a href="/camera" class="font-semibold text-terracotta hover:underline">Stuur er een</a>.
			</div>
		{:else}
			<ul class="vonk-card divide-y divide-border p-0">
				{#each data.inbox as s (s.uuid)}
					<li class="flex items-center gap-3 p-4">
						<Avatar url={s.sender.avatar_url} name={s.sender.display_name} size={48} />
						<div class="min-w-0 flex-1">
							<p class="font-bold text-ink">{s.sender.display_name}</p>
							<p class="text-sm text-muted">
								⚡ {s.view_policy === 'view_once' ? 'Eén keer kijken' : 'Blijft 24u'} · {relTime(s.created_at)}
							</p>
						</div>
						<button
							type="button"
							class="rounded-full bg-terracotta px-4 py-2 text-sm font-semibold text-white hover:bg-terracotta-dark"
							onclick={() => (viewingUuid = s.uuid)}
						>Open</button>
					</li>
				{/each}
			</ul>
		{/if}
	{:else if data.sent.length === 0}
		<div class="vonk-card text-center text-muted">Niets verzonden.</div>
	{:else}
		<ul class="vonk-card divide-y divide-border p-0">
			{#each data.sent as s (s.uuid)}
				<li class="flex items-center gap-3 p-4">
					<Avatar url={s.recipient.avatar_url} name={s.recipient.display_name} size={48} />
					<div class="min-w-0 flex-1">
						<p class="font-bold text-ink">{s.recipient.display_name}</p>
						<p class="text-sm text-muted">
							{s.viewed_by_them ? '✓ Bekeken' : '⏳ Nog niet bekeken'} · {relTime(s.created_at)}
						</p>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</main>

<Toast />

{#if viewingUuid}
	<SnapViewer uuid={viewingUuid} onClose={() => (viewingUuid = null)} />
{/if}
