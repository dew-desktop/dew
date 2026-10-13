import { getCollection } from 'astro:content';
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import type { StarlightRouteData } from '@astrojs/starlight/route-data';
import { TOPICS } from './topics.mjs';

type Entry = StarlightRouteData['sidebar'][number];
type Group = Extract<Entry, { type: 'group' }>;
type Link = Extract<Entry, { type: 'link' }>;
type Page = Awaited<ReturnType<typeof getCollection<'docs'>>>[number];

const containsCurrent = (entry: Entry): boolean =>
	entry.type === 'link' ? entry.isCurrent : entry.entries.some(containsCurrent);

const firstHref = (entry: Entry): string | undefined =>
	entry.type === 'link' ? entry.href : entry.entries.map(firstHref).find(Boolean);

const countLinks = (entry: Entry): number =>
	entry.type === 'link' ? 1 : entry.entries.reduce((n, e) => n + countLinks(e), 0);

/** `in-person` reads as "In person". */
const sentenceCase = (name: string) => {
	const words = name.replace(/-/g, ' ');
	return words.charAt(0).toUpperCase() + words.slice(1);
};

/** Pages sort by `sidebar.order` in their front matter, then by path. */
const byOrder = (a: Page, b: Page) =>
	(a.data.sidebar.order ?? Infinity) - (b.data.sidebar.order ?? Infinity) || a.id.localeCompare(b.id);

/**
 * THE NAVIGATION, BUILT FROM THE PAGES ON EVERY REQUEST. A topic is a directory
 * of `docs/` listed in `src/topics.mjs`, and each subdirectory of a topic is a
 * group. Building it here rather than in `astro.config.mjs` means a page added
 * while the dev server runs is in the navigation on the next request; a sidebar
 * computed when the config loads stays stale until a restart.
 */
function navigation(pages: Page[], currentId: string): Group[] {
	const link = (page: Page): Link => ({
		type: 'link',
		label: page.data.sidebar.label ?? page.data.title,
		href: `/${page.id}/`,
		isCurrent: page.id === currentId,
		badge: undefined,
		attrs: {},
	});
	const visible = pages.filter((page) => !page.data.sidebar.hidden).sort(byOrder);
	return TOPICS.map(({ label, directory, groups = [] }) => {
		const inTopic = visible.filter((page) => page.id.startsWith(`${directory}/`));
		const groupOf = (page: Page) => page.id.slice(directory.length + 1).split('/').slice(0, -1).join('/');
		const found = [...new Set(inTopic.map(groupOf).filter(Boolean))];
		// Listed groups first, in their order; any other subdirectory after them.
		const ordered = [...groups.filter((g) => found.includes(g)), ...found.filter((g) => !groups.includes(g)).sort()];
		const entries: Entry[] = [
			...inTopic.filter((page) => !groupOf(page)).map(link),
			...ordered.map(
				(name): Group => ({
					type: 'group',
					label: sentenceCase(name),
					entries: inTopic.filter((page) => groupOf(page) === name).map(link),
					collapsed: false,
					badge: undefined,
				}),
			),
		];
		return { type: 'group', label, entries, collapsed: false, badge: undefined } satisfies Group;
	}).filter((topic) => topic.entries.length > 0);
}

/**
 * Every topic is a tab. The page's sidebar shows only the topic it is in, and
 * the header and the breadcrumb are told which topics exist and where this page
 * sits among them. A page in no topic gets no sidebar rather than every topic at
 * once, and so does a topic with a single page, where a sidebar would list only
 * the page already open.
 */
export const onRequest = defineRouteMiddleware(async (context) => {
	const route = context.locals.starlightRoute;
	const topics = navigation(await getCollection('docs'), route.id);
	const current = topics.find(containsCurrent);

	context.locals.dew = {
		topics: topics.map((topic) => ({
			label: topic.label,
			href: firstHref(topic) ?? '/',
			current: topic === current,
		})),
		crumbs: current
			? [
					{ label: current.label, href: firstHref(current) },
					...current.entries
						.filter((entry): entry is Group => entry.type === 'group' && containsCurrent(entry))
						.map((group) => ({ label: group.label, href: undefined })),
				]
			: [],
	};

	// The home page, which belongs to no topic, lists every topic as a card.
	route.sidebar = current ? current.entries : topics;
	if (route.entry.data.template !== 'splash' && (!current || countLinks(current) < 2)) {
		route.hasSidebar = false;
	}
});
