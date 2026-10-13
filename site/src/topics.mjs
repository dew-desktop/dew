// @ts-check
// THE TOPICS, in the order the topic bar shows them. Each is a directory of
// `docs/`; `groups` lists its subdirectories in order, and `description` is the
// line the home page shows under it. `src/route-data.ts` builds the navigation
// from it on every request, and the home page reads that navigation, so the two
// cannot disagree.
//
// A topic is an area of what someone can build or use. Applets are one kind of
// Dew app; what every kind shares, such as the interface classes, is a topic of
// its own rather than a part of Applets.

/**
 * @typedef {object} Topic
 * @property {string} label What the topic bar, the breadcrumb and the home page call it.
 * @property {string} directory A directory of `docs/`.
 * @property {string} description One line, shown under the topic on the home page.
 * @property {string[]} [groups] Subdirectories that become sidebar groups, in order.
 *   Any other subdirectory is still a group, after these. `in-person` is "In person".
 */

/** @type {Topic[]} */
export const TOPICS = [
	{
		label: 'Get started',
		directory: 'get-started',
		description: 'Build and run your first applet.',
	},
	{
		label: 'Applets',
		directory: 'applets',
		description: 'Apps that people install and run on their desktop.',
	},
	{
		label: 'Interface',
		directory: 'interface',
		description: 'Build interfaces with the classes you know from Roblox.',
	},
	{
		label: 'Specification',
		directory: 'spec',
		description: 'The services every host that runs Dew apps provides.',
	},
];
