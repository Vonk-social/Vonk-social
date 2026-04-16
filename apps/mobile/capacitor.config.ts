import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
	appId: 'social.vonk.app',
	appName: 'Vonk',
	webDir: 'www',
	server: {
		// In development the device loads the live SvelteKit server.
		// In production the bundled www/ is served and API calls go
		// to vonk.social over TLS.
		url: process.env.CAPACITOR_LIVE_URL,
		cleartext: false
	},
	ios: {
		// Ensure WKWebView allows camera + mic access for the snap
		// composer on physical devices.
		allowsLinkPreview: false
	},
	android: {
		allowMixedContent: false
	},
	plugins: {
		PushNotifications: {
			presentationOptions: ['badge', 'sound', 'alert']
		},
		Camera: {
			// We always want the latest file from the system picker;
			// never cached.
			saveToGallery: false
		}
	}
};

export default config;
