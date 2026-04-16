<script lang="ts">
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import Button from '$lib/components/Button.svelte';
	import SnapViewer from '$lib/components/snaps/SnapViewer.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import { startConversation } from '$lib/api/dm';
	import { apiFetch } from '$lib/api/core';
	import { goto } from '$app/navigation';
	import { toasts } from '$lib/stores/toasts';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let tab = $state<'berichten' | 'snaps'>('berichten');
	let snapTab = $state<'inbox' | 'sent'>('inbox');
	let viewingUuid = $state<string | null>(null);

	// New conversation with autocomplete
	let showNewChat = $state(false);
	let searchQuery = $state('');
	let starting = $state(false);
	let suggestions = $state<Array<{uuid: string; username: string; display_name: string; avatar_url?: string | null}>>([]);
	let searchTimeout: ReturnType<typeof setTimeout> | null = null;

	function onSearchInput() {
		const q = searchQuery.trim();
		if (q.length < 2) {
			suggestions = [];
			return;
		}
		if (searchTimeout) clearTimeout(searchTimeout);
		searchTimeout = setTimeout(async () => {
			try {
				const res = await apiFetch(`/api/users/search?q=${encodeURIComponent(q)}&limit=6`);
				if (res.ok) {
					const body = (await res.json()) as { data: typeof suggestions };
					suggestions = body.data ?? [];
				}
			} catch { /* ignore */ }
		}, 200);
	}

	async function pickUser(username: string) {
		starting = true;
		suggestions = [];
		searchQuery = '';
		try {
			const { uuid } = await startConversation(username);
			goto(`/dm/${uuid}`);
		} catch (e) {
			toasts.push('error', (e as Error).message);
		} finally {
			starting = false;
		}
	}

	function relTime(iso: string): string {
		const diff = Math.max(0, Math.round((Date.now() - new Date(iso).getTime()) / 1000));
		if (diff < 60) return 'zojuist';
		if (diff < 3600) return `${Math.round(diff / 60)} min`;
		if (diff < 86400) return `${Math.round(diff / 3600)} u`;
		return `${Math.round(diff / 86400)} d`;
	}
</script>

<svelte:head>
	<title>Berichten — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Feed</a>

	<div class="mb-4 flex items-center justify-between">
		<h1 class="font-display text-2xl font-bold text-ink">Berichten</h1>
		<Button type="button" onclick={() => (showNewChat = !showNewChat)}>
			{showNewChat ? '✕' : '✏️ Nieuw'}
		</Button>
	</div>

	<!-- New conversation autocomplete -->
	{#if showNewChat}
		<div class="vonk-card mb-4">
			<p class="mb-2 text-sm text-muted">Zoek iemand om een gesprek te starten:</p>
			<div class="relative">
				<input
					type="text"
					bind:value={searchQuery}
					oninput={onSearchInput}
					placeholder="Typ een naam of gebruikersnaam…"
					disabled={starting}
					class="block w-full rounded-xl border border-border bg-white px-3 py-2.5 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				/>
				{#if suggestions.length > 0}
					<ul class="absolute left-0 right-0 z-10 mt-1 overflow-hidden rounded-xl border border-border bg-white shadow-lg">
						{#each suggestions as s (s.uuid)}
							<li>
								<button
									type="button"
									class="flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-border/20"
									onclick={() => pickUser(s.username)}
								>
									<Avatar url={s.avatar_url} name={s.display_name} size={36} />
									<div class="min-w-0">
										<div class="font-semibold text-ink">{s.display_name}</div>
										<div class="text-sm text-muted">@{s.username}</div>
									</div>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
			{#if starting}
				<p class="mt-2 text-sm text-muted">Gesprek starten…</p>
			{/if}
		</div>
	{/if}

	<!-- Tabs: Berichten / Snaps -->
	<div class="mb-4 flex gap-2 border-b border-border">
		<button
			type="button"
			class="px-4 py-2 text-sm font-semibold transition-colors"
			class:border-b-2={tab === 'berichten'}
			class:border-terracotta={tab === 'berichten'}
			class:text-ink={tab === 'berichten'}
			class:text-muted={tab !== 'berichten'}
			onclick={() => (tab = 'berichten')}
		>
			💬 Berichten ({data.conversations.length})
		</button>
		<button
			type="button"
			class="px-4 py-2 text-sm font-semibold transition-colors"
			class:border-b-2={tab === 'snaps'}
			class:border-terracotta={tab === 'snaps'}
			class:text-ink={tab === 'snaps'}
			class:text-muted={tab !== 'snaps'}
			onclick={() => (tab = 'snaps')}
		>
			⚡ Snaps ({data.inbox.length})
		</button>
	</div>

	{#if tab === 'berichten'}
		<!-- Conversations list -->
		{#if data.conversations.length === 0}
			<div class="vonk-card text-center text-muted">
				Nog geen berichten. Stuur iemand een bericht via de ✏️ knop hierboven.
			</div>
		{:else}
			<ul class="vonk-card divide-y divide-border p-0">
				{#each data.conversations as c (c.uuid)}
					<li>
						<a href="/dm/{c.uuid}" class="flex items-center gap-3 p-4 transition-colors hover:bg-border/20">
							<Avatar url={c.other_user.avatar_url} name={c.other_user.display_name} size={48} />
							<div class="min-w-0 flex-1">
								<div class="flex items-center justify-between">
									<p class="font-bold text-ink">{c.other_user.display_name}</p>
									{#if c.last_message_at}
										<span class="text-xs text-muted">{relTime(c.last_message_at)}</span>
									{/if}
								</div>
								<p class="truncate text-sm text-muted">
									{c.last_message ?? 'Geen berichten nog'}
								</p>
							</div>
							{#if c.unread_count > 0}
								<span class="flex h-6 w-6 items-center justify-center rounded-full bg-terracotta text-xs font-bold text-white">
									{c.unread_count}
								</span>
							{/if}
						</a>
					</li>
				{/each}
			</ul>
		{/if}
	{:else}
		<!-- Snaps -->
		<div class="mb-3 flex gap-2">
			<button
				type="button"
				class="rounded-full px-3 py-1 text-xs font-semibold transition-colors"
				class:bg-terracotta={snapTab === 'inbox'}
				class:text-white={snapTab === 'inbox'}
				class:bg-border={snapTab !== 'inbox'}
				class:text-ink={snapTab !== 'inbox'}
				onclick={() => (snapTab = 'inbox')}
			>Inbox ({data.inbox.length})</button>
			<button
				type="button"
				class="rounded-full px-3 py-1 text-xs font-semibold transition-colors"
				class:bg-terracotta={snapTab === 'sent'}
				class:text-white={snapTab === 'sent'}
				class:bg-border={snapTab !== 'sent'}
				class:text-ink={snapTab !== 'sent'}
				onclick={() => (snapTab = 'sent')}
			>Verzonden ({data.sent.length})</button>
		</div>

		{#if snapTab === 'inbox'}
			{#if data.inbox.length === 0}
				<div class="vonk-card text-center text-muted">
					Nog geen snaps. <a href="/camera" class="font-semibold text-terracotta hover:underline">Stuur er een</a>.
				</div>
			{:else}
				<ul class="vonk-card divide-y divide-border p-0">
					{#each data.inbox as s (s.uuid)}
						<li class="flex items-center gap-3 p-4">
							<Avatar url={s.sender.avatar_url} name={s.sender.display_name} size={44} />
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
						<Avatar url={s.recipient.avatar_url} name={s.recipient.display_name} size={44} />
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
	{/if}
</main>

<Toast />

{#if viewingUuid}
	<SnapViewer uuid={viewingUuid} onClose={() => (viewingUuid = null)} />
{/if}
