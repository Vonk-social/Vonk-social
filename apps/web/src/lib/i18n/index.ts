import nl from './nl.json';
import en from './en.json';
import fr from './fr.json';
import de from './de.json';
import es from './es.json';
import it from './it.json';
import pt from './pt.json';
import pl from './pl.json';
import sv from './sv.json';
import da from './da.json';
import fi from './fi.json';
import el from './el.json';
import ro from './ro.json';
import cs from './cs.json';
import uk from './uk.json';

/**
 * Available UI languages. NL + EN are the reference (native-quality). The
 * others carry reasonable translations for the landing page but may fall
 * back to NL/EN for strings that haven't been translated yet.
 */
const dictionaries: Record<string, Record<string, string>> = {
	nl,
	en,
	fr,
	de,
	es,
	it,
	pt,
	pl,
	sv,
	da,
	fi,
	el,
	ro,
	cs,
	uk
} as Record<string, Record<string, string>>;

export type Locale = keyof typeof dictionaries;

/**
 * Ordered list of supported languages — stable for UI menus / SEO.
 * Flag + native name for the language-switcher.
 */
export const SUPPORTED_LOCALES: Array<{ code: string; flag: string; name: string }> = [
	{ code: 'nl', flag: '🇳🇱', name: 'Nederlands' },
	{ code: 'en', flag: '🇬🇧', name: 'English' },
	{ code: 'fr', flag: '🇫🇷', name: 'Français' },
	{ code: 'de', flag: '🇩🇪', name: 'Deutsch' },
	{ code: 'es', flag: '🇪🇸', name: 'Español' },
	{ code: 'it', flag: '🇮🇹', name: 'Italiano' },
	{ code: 'pt', flag: '🇵🇹', name: 'Português' },
	{ code: 'pl', flag: '🇵🇱', name: 'Polski' },
	{ code: 'sv', flag: '🇸🇪', name: 'Svenska' },
	{ code: 'da', flag: '🇩🇰', name: 'Dansk' },
	{ code: 'fi', flag: '🇫🇮', name: 'Suomi' },
	{ code: 'el', flag: '🇬🇷', name: 'Ελληνικά' },
	{ code: 'ro', flag: '🇷🇴', name: 'Română' },
	{ code: 'cs', flag: '🇨🇿', name: 'Čeština' },
	{ code: 'uk', flag: '🇺🇦', name: 'Українська' }
];

/** Translate a dotted key. Falls back to NL → EN → key itself. */
export function t(key: string, locale: string | null | undefined = 'nl'): string {
	const dict = dictionaries[locale ?? 'nl'] ?? nl;
	return (
		dict[key] ??
		(nl as Record<string, string>)[key] ??
		(en as Record<string, string>)[key] ??
		key
	);
}

/** True if `x` matches one of our supported locales. */
export function isLocale(x: string | null | undefined): x is Locale {
	return typeof x === 'string' && x in dictionaries;
}

/**
 * Parse an HTTP `Accept-Language` header and pick the best supported locale.
 * Server-side only (hooks.server.ts uses this). Examples:
 *   "fr-BE,fr;q=0.9,nl;q=0.8"  → 'fr'
 *   "pt-BR,pt;q=0.9"           → 'pt'
 *   "ja"                        → 'nl' (fallback)
 */
export function pickLocaleFromHeader(header: string | null | undefined): Locale {
	if (!header) return 'nl';
	const parts = header
		.split(',')
		.map((part) => {
			const [tag, ...rest] = part.trim().split(';');
			const qStr = rest.find((s) => s.trim().startsWith('q='));
			const q = qStr ? parseFloat(qStr.split('=')[1]) : 1;
			return { tag: tag.toLowerCase(), q: isNaN(q) ? 1 : q };
		})
		.sort((a, b) => b.q - a.q);

	for (const { tag } of parts) {
		// Try full tag, then primary subtag (fr-BE → fr).
		if (isLocale(tag)) return tag as Locale;
		const primary = String(tag).split('-')[0];
		if (isLocale(primary)) return primary as Locale;
	}
	return 'nl';
}
