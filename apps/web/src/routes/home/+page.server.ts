import { redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';

export const load: PageServerLoad = ({ locals }) => {
	if (!locals.user) throw redirect(303, '/');
	if (locals.user.needs_onboarding) throw redirect(303, '/onboarding/username');
	return { user: locals.user };
};

export const actions: Actions = {
	logout: async ({ fetch, cookies }) => {
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		await fetch('/api/auth/logout', {
			method: 'POST',
			headers: { cookie: cookieHeader }
		}).catch(() => {});
		cookies.delete('vonk_access', { path: '/' });
		cookies.delete('vonk_refresh', { path: '/api/auth' });
		throw redirect(303, '/');
	}
};
