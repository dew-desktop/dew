import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import type { StarlightRouteData } from '@astrojs/starlight/route-data';

type Entry = StarlightRouteData['sidebar'][number];
type Group = Extract<Entry, { type: 'group' }>;

const containsCurrent = (entry: Entry): boolean =>
	entry.type === 'link' ? entry.isCurrent : entry.entries.some(containsCurrent);

const firstHref = (entry: Entry): string | undefined =>
	entry.type === 'link' ? entry.href : entry.entries.map(firstHref).find(Boolean);

/**
 * Every top-level sidebar group is a topic. The page sees only the topic it is
 * in, and the header and the breadcrumb are told which topics exist and where
 * this page sits among them.
 */
export const onRequest = defineRouteMiddleware((context) => {
	const route = context.locals.starlightRoute;
	const topics = route.sidebar.filter((entry): entry is Group => entry.type === 'group');
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

	if (current) route.sidebar = current.entries;
});
