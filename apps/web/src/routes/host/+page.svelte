<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import { toasts } from '$lib/stores/toasts';
	import type { PageProps } from './$types';

	let { form }: PageProps = $props();
	let submitted = $state(false);

	$effect(() => {
		if (form && 'ok' in form && form.ok) {
			submitted = true;
			toasts.push('success', 'Aanvraag ingediend!');
		} else if (form && 'error' in form) {
			toasts.push('error', String(form.error));
		}
	});
</script>

<svelte:head>
	<title>Host een Vonk-node — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-12">
	<h1 class="mb-2 font-display text-3xl font-bold text-ink">Word een Vonk-host</h1>
	<p class="mb-8 text-lg text-muted">
		Vonk is een gedistribueerd platform. Jouw data zit niet op één server — het wordt
		versleuteld verspreid over meerdere nodes, zoals RAID bij harde schijven. Als er eentje
		uitvalt, draait de rest gewoon door.
	</p>

	<Card class="mb-8">
		<h2 class="mb-3 font-display text-xl font-bold text-ink">Hoe werkt het?</h2>
		<div class="space-y-4 text-sm text-ink">
			<div class="flex gap-3">
				<span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-terracotta text-sm font-bold text-white">1</span>
				<div>
					<p class="font-semibold">Jij biedt een server aan</p>
					<p class="text-muted">Een VPS van €5/maand is genoeg. Docker moet draaien. Wij leveren de images.</p>
				</div>
			</div>
			<div class="flex gap-3">
				<span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-terracotta text-sm font-bold text-white">2</span>
				<div>
					<p class="font-semibold">Wij keuren je aanvraag goed</p>
					<p class="text-muted">Je krijgt een unieke API-sleutel. Eén commando en je node joint het netwerk.</p>
				</div>
			</div>
			<div class="flex gap-3">
				<span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-terracotta text-sm font-bold text-white">3</span>
				<div>
					<p class="font-semibold">Data wordt automatisch verdeeld</p>
					<p class="text-muted">
						Gebruikersdata wordt versleuteld gerepliceerd over meerdere nodes.
						DM's en snaps zijn end-to-end versleuteld — zelfs jij als host kan ze niet lezen.
					</p>
				</div>
			</div>
			<div class="flex gap-3">
				<span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-terracotta text-sm font-bold text-white">4</span>
				<div>
					<p class="font-semibold">Jouw node maakt het netwerk sterker</p>
					<p class="text-muted">Meer nodes = meer capaciteit, meer redundantie, lagere kosten per persoon. Net als torrent-seeders.</p>
				</div>
			</div>
		</div>
	</Card>

	<Card class="mb-8">
		<h2 class="mb-3 font-display text-xl font-bold text-ink">Wat je NIET kan zien als host</h2>
		<ul class="list-disc space-y-1 pl-5 text-sm text-muted">
			<li>DM's en snaps (end-to-end versleuteld, private key zit in de browser van de gebruiker)</li>
			<li>Likes (worden nooit via de API getoond, zelfs niet aan hosts)</li>
			<li>Wachtwoorden (er zijn er geen — alleen OAuth via Google/GitHub/Apple)</li>
			<li>IP-adressen (worden gehashed en na 48 uur gewist)</li>
		</ul>
	</Card>

	{#if submitted}
		<Card>
			<div class="text-center">
				<p class="text-2xl">🎉</p>
				<h2 class="mt-2 font-display text-xl font-bold text-ink">Bedankt!</h2>
				<p class="mt-2 text-muted">Je aanvraag is ingediend. We nemen zo snel mogelijk contact op via e-mail.</p>
			</div>
		</Card>
	{:else}
		<Card>
			<h2 class="mb-4 font-display text-xl font-bold text-ink">Aanvraag indienen</h2>
			<form method="POST" class="flex flex-col gap-4">
				<label class="text-sm font-semibold text-ink">
					Naam *
					<input type="text" name="name" required maxlength="100"
						class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30" />
				</label>
				<label class="text-sm font-semibold text-ink">
					E-mail *
					<input type="email" name="email" required maxlength="254"
						class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30" />
				</label>
				<label class="text-sm font-semibold text-ink">
					Waarom wil je een node hosten?
					<textarea name="note" rows={3} maxlength="1000"
						class="mt-1 block w-full resize-none rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30"></textarea>
				</label>
				<div class="grid grid-cols-2 gap-3">
					<label class="text-sm font-semibold text-ink">
						Regio
						<input type="text" name="region" placeholder="bv. eu-west" maxlength="50"
							class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30" />
					</label>
					<label class="text-sm font-semibold text-ink">
						Server URL
						<input type="text" name="url" placeholder="https://mijn-node.example.com" maxlength="500"
							class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30" />
					</label>
				</div>
				<fieldset>
					<legend class="mb-1 text-sm font-semibold text-ink">Server specs (optioneel)</legend>
					<div class="grid grid-cols-3 gap-3">
						<label class="text-sm text-muted">
							CPU cores
							<input type="number" name="cpu_cores" min="1" max="999"
								class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30" />
						</label>
						<label class="text-sm text-muted">
							RAM (GB)
							<input type="number" name="ram_gb" min="1" max="9999"
								class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30" />
						</label>
						<label class="text-sm text-muted">
							Disk (GB)
							<input type="number" name="disk_gb" min="1" max="99999"
								class="mt-1 block w-full rounded-xl border border-border bg-white px-3 py-2 text-ink focus:border-terracotta focus:outline-none focus:ring-2 focus:ring-terracotta/30" />
						</label>
					</div>
				</fieldset>
				<div class="self-end">
					<Button type="submit">Aanvraag versturen</Button>
				</div>
			</form>
		</Card>
	{/if}
</main>

<Toast />
