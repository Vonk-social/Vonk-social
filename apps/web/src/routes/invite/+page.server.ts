import { fail, redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';
import { apiFetch } from '$lib/api/core';

export const load: PageServerLoad = async ({ locals, cookies }) => {
	if (!locals.user) throw redirect(303, '/login');
	const cookieHeader = cookies
		.getAll()
		.map((c) => `${c.name}=${c.value}`)
		.join('; ');
	try {
		const res = await apiFetch('/api/invites/sent', { method: 'GET', cookies: cookieHeader });
		if (res.ok) {
			const body = (await res.json()) as { data: unknown[] };
			return { user: locals.user, sent: body.data ?? [] };
		}
	} catch {
		// fallthrough
	}
	return { user: locals.user, sent: [] as unknown[] };
};

export const actions: Actions = {
	invite: async ({ request, cookies }) => {
		const data = await request.formData();
		const email = String(data.get('email') ?? '').trim();
		const note = String(data.get('note') ?? '').trim();
		if (!email || !email.includes('@')) {
			return fail(400, { error: 'Ongeldig e-mailadres' });
		}
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			const res = await apiFetch('/api/invites', {
				method: 'POST',
				cookies: cookieHeader,
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ email, note: note || undefined })
			});
			if (!res.ok) {
				const err = (await res.json().catch(() => ({}))) as {
					error?: { code?: string; message?: string };
				};
				return fail(res.status, { error: err.error?.message ?? `HTTP ${res.status}` });
			}
			const body = (await res.json()) as { data: { sent: boolean; message: string } };
			return { ok: true, sent: body.data.sent, message: body.data.message };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	},

	matchHandles: async ({ request, cookies }) => {
		const data = await request.formData();
		const body = {
			instagram: parseList(data.get('instagram')),
			twitter: parseList(data.get('twitter')),
			snapchat: parseList(data.get('snapchat')),
			telegram: parseList(data.get('telegram')),
			bluesky: parseList(data.get('bluesky')),
			mastodon: parseList(data.get('mastodon'))
		};
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			const res = await apiFetch('/api/invites/match-handles', {
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
			const parsed = (await res.json()) as { data: unknown[] };
			return { ok: true, matches: parsed.data ?? [] };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	}
};

function parseList(raw: FormDataEntryValue | null): string[] {
	if (!raw) return [];
	return String(raw)
		.split(/[\s,]+/)
		.map((s) => s.trim())
		.filter((s) => s.length > 0);
}
