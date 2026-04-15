import { redirect } from '@sveltejs/kit';
import type { Actions } from './$types';
import { patchMe } from '$lib/api';

export const actions: Actions = {
	done: async ({ cookies }) => {
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		await patchMe({ finish_onboarding: true }, cookieHeader).catch(() => {});
		throw redirect(303, '/home');
	}
};
