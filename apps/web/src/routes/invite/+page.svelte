<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import { toasts } from '$lib/stores/toasts';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();

	type SentInvite = {
		uuid: string;
		recipient_email: string;
		note?: string | null;
		sent_at?: string | null;
		accepted_at?: string | null;
		failed_at?: string | null;
		failure_reason?: string | null;
		created_at: string;
	};
	type MatchedUser = {
		uuid: string;
		username: string;
		display_name: string;
		avatar_url?: string | null;
		matched_on: string;
	};

	const sent = $derived((data.sent as SentInvite[]) ?? []);
	const matches = $derived(
		form && 'matches' in form ? ((form as { matches: MatchedUser[] }).matches ?? []) : []
	);

	$effect(() => {
		if (form && 'ok' in form && form.ok && 'sent' in form && form.sent) {
			toasts.push('success', 'Uitnodiging verstuurd');
		} else if (form && 'ok' in form && form.ok && 'message' in form && form.message === 'queued') {
			toasts.push('info', 'In de wachtrij — SMTP is nog niet geconfigureerd.');
		} else if (form && 'error' in form && form.error) {
			toasts.push('error', String(form.error));
		}
	});
</script>

<svelte:head>
	<title>Vrienden uitnodigen — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">
		← Feed
	</a>
	<h1 class="mb-2 font-display text-2xl font-bold text-ink">Nodig je vrienden uit</h1>
	<p class="mb-6 text-muted">
		Vonk groeit enkel via uitnodigingen van vrienden — geen reclame, geen groei-hacking.
	</p>

	<Card class="mb-6">
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Per e-mail</h2>
		<form method="POST" action="?/invite" class="flex flex-col gap-3">
			<label class="text-sm font-semibold text-ink">
				E-mailadres
				<input
					type="email"
					name="email"
					required
					class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				/>
			</label>
			<label class="text-sm font-semibold text-ink">
				Persoonlijk bericht (optioneel)
				<textarea
					name="note"
					rows={2}
					maxlength={500}
					class="mt-1 block w-full resize-none rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
				></textarea>
			</label>
			<div class="self-end">
				<Button type="submit">Uitnodiging versturen</Button>
			</div>
		</form>
	</Card>

	<Card class="mb-6">
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Vind vrienden via andere platforms</h2>
		<p class="mb-4 text-sm text-muted">
			Plak de handvatten van je vrienden op andere platforms. We matchen enkel tegen Vonk-gebruikers
			die die handvatten zelf op hun profiel hebben gezet. Meerdere tegelijk mag — scheid met spatie
			of komma.
		</p>
		<form method="POST" action="?/matchHandles" class="grid grid-cols-1 gap-3 sm:grid-cols-2">
			{#each [
				['instagram', 'Instagram'],
				['twitter', 'X / Twitter'],
				['snapchat', 'Snapchat'],
				['telegram', 'Telegram'],
				['bluesky', 'Bluesky'],
				['mastodon', 'Mastodon']
			] as [name, label] (name)}
				<label class="text-sm font-semibold text-ink">
					{label}
					<input
						type="text"
						{name}
						placeholder="@vriend1 @vriend2"
						class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"
					/>
				</label>
			{/each}
			<div class="col-span-full self-end">
				<Button type="submit">Zoeken</Button>
			</div>
		</form>

		{#if matches.length > 0}
			<div class="mt-6">
				<h3 class="mb-3 text-sm font-semibold text-ink">{matches.length} match{matches.length === 1 ? '' : 'es'}:</h3>
				<ul class="flex flex-col gap-3">
					{#each matches as m (m.uuid)}
						<li class="flex items-center justify-between gap-3 rounded-xl border border-border bg-white p-3">
							<a href="/u/{m.username}" class="flex items-center gap-3">
								<Avatar url={m.avatar_url ?? null} name={m.display_name} size={40} />
								<div>
									<div class="font-semibold text-ink">{m.display_name}</div>
									<div class="text-sm text-muted">@{m.username} · match via {m.matched_on}</div>
								</div>
							</a>
						</li>
					{/each}
				</ul>
			</div>
		{:else if form && 'ok' in form && form.ok && 'matches' in form}
			<p class="mt-4 text-sm text-muted">Geen matches gevonden — misschien gebruiken ze nog geen Vonk?</p>
		{/if}
	</Card>

	{#if sent.length > 0}
		<Card>
			<h2 class="mb-3 font-display text-lg font-bold text-ink">Verstuurde uitnodigingen</h2>
			<ul class="flex flex-col gap-2">
				{#each sent as s (s.uuid)}
					<li class="flex items-center justify-between rounded-xl border border-border bg-white p-3">
						<div>
							<div class="font-semibold text-ink">{s.recipient_email}</div>
							<div class="text-xs text-muted">
								{#if s.accepted_at}
									Geaccepteerd op {new Date(s.accepted_at).toLocaleDateString()}
								{:else if s.failed_at}
									Mislukt: {s.failure_reason ?? 'onbekende fout'}
								{:else if s.sent_at}
									Verzonden op {new Date(s.sent_at).toLocaleDateString()}
								{:else}
									In wachtrij…
								{/if}
							</div>
						</div>
					</li>
				{/each}
			</ul>
		</Card>
	{/if}
</main>

<Toast />
