// See https://svelte.dev/docs/kit/types#app
import type { MeProfile } from '$lib/api';

declare global {
	namespace App {
		interface Error {}
		interface Locals {
			/** Populated by `hooks.server.ts` when a valid session cookie is present. */
			user: MeProfile | null;
			/** Resolved locale: cookie → Accept-Language → 'nl'. */
			locale: string;
		}
		interface PageData {
			user: MeProfile | null;
			locale: string;
		}
		interface PageState {}
		interface Platform {}
	}
}

export {};
