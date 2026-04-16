import { fail, redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';
import { apiFetch } from '$lib/api/core';

export const load: PageServerLoad = async ({ locals, request }) => {
	if (!locals.user) throw redirect(303, '/login');
	const cookies = request.headers.get('cookie') ?? '';

	const [nodesRes, requestsRes] = await Promise.all([
		apiFetch('/api/admin/nodes', { method: 'GET', cookies }).catch(() => null),
		apiFetch('/api/admin/nodes/requests', { method: 'GET', cookies }).catch(() => null)
	]);

	let nodes: unknown[] = [];
	let requests: unknown[] = [];

	if (nodesRes?.ok) {
		const body = (await nodesRes.json()) as { data: unknown[] };
		nodes = body.data ?? [];
	}
	if (requestsRes?.ok) {
		const body = (await requestsRes.json()) as { data: unknown[] };
		requests = body.data ?? [];
	}

	return { user: locals.user, nodes, requests };
};

export const actions: Actions = {
	approve: async ({ request, cookies }) => {
		const data = await request.formData();
		const uuid = String(data.get('uuid') ?? '');
		const note = String(data.get('note') ?? '').trim();
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			const res = await apiFetch(`/api/admin/nodes/requests/${uuid}/approve`, {
				method: 'POST',
				cookies: cookieHeader,
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ note: note || undefined })
			});
			if (!res.ok) {
				const err = (await res.json().catch(() => ({}))) as {
					error?: { message?: string };
				};
				return fail(res.status, { error: err.error?.message ?? `HTTP ${res.status}` });
			}
			const body = (await res.json()) as { data: { node_id: string; api_key: string } };
			return { approved: true, node_id: body.data.node_id, api_key: body.data.api_key };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	},
	reject: async ({ request, cookies }) => {
		const data = await request.formData();
		const uuid = String(data.get('uuid') ?? '');
		const reason = String(data.get('reason') ?? '').trim();
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			const res = await apiFetch(`/api/admin/nodes/requests/${uuid}/reject`, {
				method: 'POST',
				cookies: cookieHeader,
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ reason: reason || undefined })
			});
			if (!res.ok) {
				const err = (await res.json().catch(() => ({}))) as {
					error?: { message?: string };
				};
				return fail(res.status, { error: err.error?.message ?? `HTTP ${res.status}` });
			}
			return { rejected: true };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	},
	drain: async ({ request, cookies }) => {
		const data = await request.formData();
		const id = String(data.get('id') ?? '');
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		await apiFetch(`/api/admin/nodes/${id}/drain`, {
			method: 'POST',
			cookies: cookieHeader
		});
		return { drained: true };
	}
};
