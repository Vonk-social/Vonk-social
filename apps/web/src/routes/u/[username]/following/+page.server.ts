import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchFollowing } from '$lib/api/follows';

export const load: PageServerLoad = async ({ locals, params, request }) => {
	if (!locals.user) throw redirect(303, '/');
	const cookies = request.headers.get('cookie') ?? '';
	const list = await fetchFollowing(params.username, { limit: 50 }, cookies).catch(() => ({
		data: [],
		cursor: null,
		has_more: false
	}));
	return { user: locals.user, username: params.username, list, title: 'Volgend' };
};
