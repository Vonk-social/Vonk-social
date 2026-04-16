import { fail, redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';
import { patchMe } from '$lib/api/core';

export const load: PageServerLoad = ({ locals }) => {
	if (!locals.user) throw redirect(303, '/');
	return { user: locals.user };
};

export const actions: Actions = {
	savePrivacy: async ({ request, cookies }) => {
		const data = await request.formData();
		const is_private = data.get('is_private') === 'on';
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			await patchMe({ is_private }, cookieHeader);
			return { saved: true };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	},
	saveProfile: async ({ request, cookies }) => {
		const data = await request.formData();
		const display_name = String(data.get('display_name') ?? '').trim();
		const bio = String(data.get('bio') ?? '');
		const location_city = String(data.get('location_city') ?? '').trim();
		const location_country = String(data.get('location_country') ?? '').trim();
		const locale = String(data.get('locale') ?? 'nl');
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			await patchMe(
				{
					display_name: display_name || undefined,
					bio,
					location_city: location_city || undefined,
					location_country: location_country || undefined,
					locale
				},
				cookieHeader
			);
			return { saved: true };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	},
	saveHandles: async ({ request, cookies }) => {
		const data = await request.formData();
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		try {
			await patchMe(
				{
					handle_instagram: String(data.get('handle_instagram') ?? ''),
					handle_twitter: String(data.get('handle_twitter') ?? ''),
					handle_snapchat: String(data.get('handle_snapchat') ?? ''),
					handle_telegram: String(data.get('handle_telegram') ?? ''),
					handle_bluesky: String(data.get('handle_bluesky') ?? ''),
					handle_mastodon: String(data.get('handle_mastodon') ?? ''),
					handle_website: String(data.get('handle_website') ?? '')
				},
				cookieHeader
			);
			return { saved: true };
		} catch (e) {
			return fail(500, { error: (e as Error).message });
		}
	},
	logoutEverywhere: async ({ fetch, cookies }) => {
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		// Phase 1 logout invalidates just this session; multi-session logout
		// ships with the sessions UI in Phase 2.5.
		await fetch('/api/auth/logout', {
			method: 'POST',
			headers: { cookie: cookieHeader }
		}).catch(() => {});
		cookies.delete('vonk_access', { path: '/' });
		cookies.delete('vonk_refresh', { path: '/api/auth' });
		throw redirect(303, '/');
	}
};
