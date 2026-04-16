import type { LayoutServerLoad } from './$types';
import { apiFetch } from '$lib/api/core';

export const load: LayoutServerLoad = async ({ locals, request }) => {
	let unreadCount = 0;
	if (locals.user) {
		try {
			const cookies = request.headers.get('cookie') ?? '';
			const res = await apiFetch('/api/dm/conversations', { cookies });
			if (res.ok) {
				const body = (await res.json()) as {
					data: Array<{ unread_count: number }>;
				};
				unreadCount = body.data.reduce((sum, c) => sum + (c.unread_count ?? 0), 0);
			}
		} catch {
			// ignore — badge just won't show
		}
	}
	return { user: locals.user, locale: locals.locale, unreadCount };
};
