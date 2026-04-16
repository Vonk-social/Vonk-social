<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import Card from '$lib/components/Card.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import { toasts } from '$lib/stores/toasts';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();

	type Node = {
		id: string;
		name: string;
		api_url: string;
		region?: string;
		status: string;
		cpu_usage?: number;
		memory_usage?: number;
		disk_usage?: number;
		user_count?: number;
		last_heartbeat?: string;
		created_at: string;
	};
	type JoinReq = {
		uuid: string;
		name: string;
		email: string;
		note?: string;
		proposed_region?: string;
		proposed_url?: string;
		cpu_cores?: number;
		ram_gb?: number;
		disk_gb?: number;
		status: string;
		created_at: string;
	};

	const nodes = $derived((data.nodes as Node[]) ?? []);
	const requests = $derived((data.requests as JoinReq[]) ?? []);
	const pendingRequests = $derived(requests.filter((r) => r.status === 'pending'));

	$effect(() => {
		if (form && 'approved' in form && form.approved) {
			toasts.push('success', `Node goedgekeurd. API key: ${(form as { api_key: string }).api_key}`);
		} else if (form && 'rejected' in form) {
			toasts.push('success', 'Aanvraag afgewezen.');
		} else if (form && 'error' in form) {
			toasts.push('error', String(form.error));
		}
	});

	function statusColor(s: string): string {
		switch (s) {
			case 'active': return 'text-sage';
			case 'joining': case 'syncing': return 'text-amber';
			case 'draining': return 'text-orange-500';
			case 'dead': return 'text-red-500';
			default: return 'text-muted';
		}
	}

	function pct(v: number | undefined): string {
		if (v == null) return '—';
		return `${Math.round(v * 100)}%`;
	}
</script>

<svelte:head>
	<title>Node beheer — Vonk Admin</title>
</svelte:head>

<main class="mx-auto max-w-4xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Home</a>
	<h1 class="mb-6 font-display text-2xl font-bold text-ink">🖥️ Cluster beheer</h1>

	<!-- Pending requests -->
	{#if pendingRequests.length > 0}
		<section class="mb-8">
			<h2 class="mb-3 font-display text-lg font-bold text-ink">
				Wachtende aanvragen ({pendingRequests.length})
			</h2>
			{#each pendingRequests as req (req.uuid)}
				<Card class="mb-3">
					<div class="flex flex-wrap items-start justify-between gap-4">
						<div class="min-w-0 flex-1">
							<div class="font-bold text-ink">{req.name}</div>
							<div class="text-sm text-muted">{req.email}</div>
							{#if req.note}
								<p class="mt-2 text-sm text-ink">&ldquo;{req.note}&rdquo;</p>
							{/if}
							<div class="mt-2 flex flex-wrap gap-3 text-xs text-muted">
								{#if req.proposed_region}<span>📍 {req.proposed_region}</span>{/if}
								{#if req.proposed_url}<span>🔗 {req.proposed_url}</span>{/if}
								{#if req.cpu_cores}<span>{req.cpu_cores} cores</span>{/if}
								{#if req.ram_gb}<span>{req.ram_gb} GB RAM</span>{/if}
								{#if req.disk_gb}<span>{req.disk_gb} GB disk</span>{/if}
							</div>
						</div>
						<div class="flex gap-2">
							<form method="POST" action="?/approve">
								<input type="hidden" name="uuid" value={req.uuid} />
								<Button type="submit">✅ Goedkeuren</Button>
							</form>
							<form method="POST" action="?/reject">
								<input type="hidden" name="uuid" value={req.uuid} />
								<Button type="submit" variant="ghost">❌ Afwijzen</Button>
							</form>
						</div>
					</div>
				</Card>
			{/each}
		</section>
	{/if}

	<!-- Active nodes -->
	<section>
		<h2 class="mb-3 font-display text-lg font-bold text-ink">
			Nodes ({nodes.length})
		</h2>
		{#if nodes.length === 0}
			<Card>
				<p class="text-center text-muted">
					Nog geen nodes in het cluster. Dit is de hoofdnode.
					Vrijwilligers kunnen zich aanmelden via <a href="/host" class="text-terracotta hover:underline">/host</a>.
				</p>
			</Card>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-border text-left text-xs font-semibold uppercase text-muted">
							<th class="pb-2 pr-4">Node</th>
							<th class="pb-2 pr-4">Status</th>
							<th class="pb-2 pr-4">CPU</th>
							<th class="pb-2 pr-4">RAM</th>
							<th class="pb-2 pr-4">Disk</th>
							<th class="pb-2 pr-4">Users</th>
							<th class="pb-2 pr-4">Heartbeat</th>
							<th class="pb-2"></th>
						</tr>
					</thead>
					<tbody>
						{#each nodes as n (n.id)}
							<tr class="border-b border-border/50">
								<td class="py-3 pr-4">
									<div class="font-semibold text-ink">{n.name}</div>
									<div class="text-xs text-muted">{n.region ?? '—'}</div>
								</td>
								<td class="py-3 pr-4">
									<span class="font-semibold {statusColor(n.status)}">{n.status}</span>
								</td>
								<td class="py-3 pr-4 tabular-nums">{pct(n.cpu_usage)}</td>
								<td class="py-3 pr-4 tabular-nums">{pct(n.memory_usage)}</td>
								<td class="py-3 pr-4 tabular-nums">{pct(n.disk_usage)}</td>
								<td class="py-3 pr-4 tabular-nums">{n.user_count ?? '—'}</td>
								<td class="py-3 pr-4 text-xs text-muted">
									{n.last_heartbeat
										? new Date(n.last_heartbeat).toLocaleTimeString()
										: 'nooit'}
								</td>
								<td class="py-3">
									{#if n.status === 'active'}
										<form method="POST" action="?/drain" class="inline">
											<input type="hidden" name="id" value={n.id} />
											<button type="submit"
												class="text-xs font-semibold text-muted hover:text-red-500"
											>Drain</button>
										</form>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>
</main>

<Toast />
