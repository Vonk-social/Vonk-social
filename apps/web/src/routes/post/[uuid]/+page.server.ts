import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchPost, fetchReplies } from '$lib/api/posts';

export const load: PageServerLoad = async ({ locals, params, request }) => {
	if (!locals.user) throw redirect(303, '/');
	const cookies = request.headers.get('cookie') ?? '';
	try {
		const post = await fetchPost(params.uuid, cookies);
		const replies = await fetchReplies(params.uuid, { limit: 20 }, cookies).catch(() => ({
			data: [],
			cursor: null,
			has_more: false
		}));
		return { user: locals.user, post, replies };
	} catch (e) {
		const err = e as Error & { status?: number };
		throw error(err.status === 404 ? 404 : 500, err.message ?? 'kon post niet laden');
	}
};
