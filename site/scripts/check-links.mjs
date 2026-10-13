// @ts-check
// Fails when a built page links to a page or heading the build did not produce.
//
// It reads `dist/` rather than the sources, so a link is checked however it was
// written: in Markdown, rewritten from a relative `.md` path, or by hand in an
// `.astro` page.
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const DIST = fileURLToPath(new URL('../dist/', import.meta.url));

/** @param {string} dir @returns {string[]} */
const pages = (dir) =>
	readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) return entry.name === '_astro' || entry.name === 'pagefind' ? [] : pages(full);
		return entry.name.endsWith('.html') ? [full] : [];
	});

/** @type {Map<string, Set<string>>} */
const ids = new Map();
/** @param {string} file */
const idsIn = (file) => {
	let found = ids.get(file);
	if (!found) {
		found = new Set([...readFileSync(file, 'utf8').matchAll(/\sid="([^"]+)"/g)].map((m) => m[1]));
		ids.set(file, found);
	}
	return found;
};

/** @param {string} route */
const fileFor = (route) => {
	const target = path.join(DIST, decodeURIComponent(route));
	for (const candidate of [path.join(target, 'index.html'), target, `${target}.html`]) {
		if (existsSync(candidate) && candidate.endsWith('.html')) return candidate;
	}
	return existsSync(target) ? target : undefined;
};

const broken = [];
for (const page of pages(DIST)) {
	const html = readFileSync(page, 'utf8');
	for (const [, href] of html.matchAll(/\shref="([^"]*)"/g)) {
		if (/^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(href)) continue;
		const [route, hash] = href.split('#');
		const absolute = route === '' ? undefined : new URL(route, `http://site/${path.relative(DIST, page).replace(/\\/g, '/')}`).pathname;
		const file = absolute === undefined ? page : fileFor(absolute);
		if (!file) broken.push(`${path.relative(DIST, page)} -> ${href} (no such page)`);
		else if (hash && file.endsWith('.html') && !idsIn(file).has(decodeURIComponent(hash)))
			broken.push(`${path.relative(DIST, page)} -> ${href} (no such heading)`);
	}
}

// EVERY PAGE IN `docs/` IS IN THE NAVIGATION. The sidebar is on every one of
// those pages, so each of them has to link to all the others. A sidebar that
// builds and lists nothing passes every check above.
const DOCS = fileURLToPath(new URL('../../docs/', import.meta.url));
/** @param {string} dir @returns {string[]} */
const markdown = (dir) =>
	readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) return markdown(full);
		return entry.name.endsWith('.md') && !entry.name.startsWith('_') ? [full] : [];
	});
const routes = markdown(DOCS).map((file) => `/${path.relative(DOCS, file).replace(/\\/g, '/').replace(/\.md$/, '')}/`);
for (const route of routes) {
	const file = fileFor(route);
	if (!file) continue;
	const html = readFileSync(file, 'utf8');
	for (const other of routes) {
		if (!html.includes(`href="${other}"`)) broken.push(`${route} does not link ${other} (missing from the navigation)`);
	}
}

if (broken.length) {
	console.error(`${broken.length} broken link(s):\n  ${[...new Set(broken)].join('\n  ')}`);
	process.exit(1);
}
console.log(`links: every internal link in ${pages(DIST).length} page(s) resolves, and every page is in the navigation`);
