import { useEffect, useState } from "react";

export type TocSection = {
  id: string;
  title: string;
};

export default function TableOfContents({ sections }: { sections: TocSection[] }) {
  const [active, setActive] = useState(sections[0]?.id ?? "");

  useEffect(() => {
    const elements = sections
      .map((section) => document.getElementById(section.id))
      .filter((element): element is HTMLElement => element !== null);

    if (elements.length === 0) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);

        if (visible[0]) setActive(visible[0].target.id);
      },
      { rootMargin: "-96px 0px -65% 0px" },
    );

    elements.forEach((element) => observer.observe(element));

    return () => observer.disconnect();
  }, [sections]);

  return (
    <aside className="sticky top-24 hidden h-fit w-[210px] shrink-0 xl:block">
      <h2 className="mb-5 text-[10px] font-semibold uppercase tracking-[0.2em] text-faint">
        On this page
      </h2>
      <nav aria-label="On this page" className="flex flex-col border-l border-line">
        {sections.map((section) => (
          <a
            key={section.id}
            href={`#${section.id}`}
            className={`-ml-px border-l py-2 pl-4 text-xs leading-5 transition-colors ${
              active === section.id
                ? "border-emphasis text-ink"
                : "border-transparent text-muted hover:text-ink"
            }`}
          >
            {section.title}
          </a>
        ))}
      </nav>
    </aside>
  );
}
