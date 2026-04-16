import { fail } from '@sveltejs/kit';
import type { Actions } from './$types';
import { apiFetch } from '$lib/api/core';

export const actions: Actions = {
	default: async ({ request, cookies }) => {
		const data = await request.formData();
		const name = String(data.get('name') ?? '').trim();
		const email = String(data.get('email') ?? '').trim();
		if (!name || !email) {
			return fail(400, { error: 'Naam en e-mail zijn verplicht.' });
		}
		const body = {
			name,
			email,
			note: String(data.get('note') ?? '').trim() || undefined,
			proposed_region: String(data.get('region') ?? '').trim() || undefined,
			proposed_url: String(data.get('url') ?? '').trim() || undefined,
			cpu_cores: data.get('cpu_cores') ? Number(data.get('cpu_cores')) : undefined,
			ram_gb: data.get('ram_gb') ? Number(data.get('ram_gb')) : undefined,
			disk_gb: data.get('disk_gb') ? Number(data.get('disk_gb')) : undefined
		};
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			const res = await apiFetch('/api/cluster/join-request', {
				method: 'POST',
				cookies: cookieHeader,
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(body)
			});
			if (!res.ok) {
				const err = (await res.json().catch(() => ({}))) as {
					error?: { message?: string };
				};
				return fail(res.status, { error: err.error?.message ?? `HTTP ${res.status}` });
			}
			return { ok: true };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	}
};
