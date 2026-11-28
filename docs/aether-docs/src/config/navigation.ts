export type DocPage = {
  title: string;
  href: string;
};

export type NavGroup = {
  title: string;
  items: DocPage[];
};

export const navigation: NavGroup[] = [
  {
    title: "Getting started",
    items: [
      { title: "Introduction", href: "/docs/introduction" },
    ],
  },
  // {
  //   title: "Guides",
  //   items: [
  //     { title: "Configuration", href: "/docs/configuration" },
  //     { title: "Workspaces", href: "/docs/workspaces" },
  //   ],
  // },
  {
    title: "Reference",
    items: [{ title: "Architecture", href: "/docs/architecture" }],
  },
];

export const pages = navigation.flatMap((group) => group.items);

export function findPage(href: string) {
  for (const group of navigation) {
    const page = group.items.find((item) => item.href === href);
    if (page) {
      return { page, group: group.title };
    }
  }

  return { page: pages[0], group: navigation[0].title };
}

export function getPageNeighbors(href: string) {
  const index = pages.findIndex((page) => page.href === href);

  return {
    previous: index > 0 ? pages[index - 1] : null,
    next: index >= 0 && index < pages.length - 1 ? pages[index + 1] : null,
  };
}
