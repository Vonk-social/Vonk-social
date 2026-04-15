import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

/**
 * Dev: the Vite dev server proxies `/api/*` to the Rust backend on :8080 so
 * the browser sees one origin. This keeps cookies/CORS simple and mirrors the
 * production topology (nginx → API + SvelteKit node).
 */
export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		host: '0.0.0.0',
		port: 5173,
		proxy: {
			'/api': {
				target: 'http://localhost:8080',
				changeOrigin: false,
				xfwd: true
			},
			'/media': {
				target: 'http://localhost:9100',
				changeOrigin: true,
				rewrite: (path) => path.replace(/^\/media/, '/vonk-media')
			}
		}
	}
});
