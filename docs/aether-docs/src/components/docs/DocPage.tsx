import type { ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";

import TableOfContents, { type TocSection } from "./TableOfContents";
import { findPage, getPageNeighbors } from "../../config/navigation";

type DocPageProps = {
  eyebrow: string;
  title: string;
  description: string;
  sections: TocSection[];
  children: ReactNode;
};

export default function DocPage({ eyebrow, title, description, sections, children }: DocPageProps) {
  const { pathname } = useLocation();
  const { group } = findPage(pathname);
  const { previous, next } = getPageNeighbors(pathname);

  return (
    <div className="mx-auto w-full max-w-[1180px] px-5 py-12 sm:px-8 lg:px-12 lg:py-16">
      <div className="flex gap-16 xl:gap-20">
        <article className="min-w-0 max-w-[780px] flex-1">
          <header>
            <div className="mb-8 flex flex-wrap items-center gap-2 text-xs text-faint">
              <span>Docs</span>
              <span aria-hidden="true">/</span>
              <span>{group}</span>
              <span aria-hidden="true">/</span>
              <span className="text-muted">{title}</span>
            </div>

            <p className="mb-4 text-[10px] font-semibold uppercase tracking-[0.22em] text-emphasis">
              {eyebrow}
            </p>

            <h1 className="font-display text-4xl font-semibold tracking-tight sm:text-5xl lg:text-6xl">
              {title}
            </h1>

            <p className="mt-6 max-w-2xl text-base leading-7 text-muted sm:text-lg">{description}</p>
          </header>

          <div className="mt-12">{children}</div>

          <nav
            aria-label="Page navigation"
            className="mt-20 grid gap-3 border-t border-line pt-8 sm:grid-cols-2"
          >
            {previous ? (
              <Link
                to={previous.href}
                className="group flex flex-col gap-1 border border-line px-5 py-4 transition-colors hover:border-emphasis/40 hover:bg-panel"
              >
                <span className="text-[10px] uppercase tracking-[0.18em] text-faint">Previous</span>
                <span className="text-sm text-muted transition-colors group-hover:text-emphasis">
                  ← {previous.title}
                </span>
              </Link>
            ) : (
              <span />
            )}

            {next && (
              <Link
                to={next.href}
                className="group flex flex-col gap-1 border border-line px-5 py-4 text-right transition-colors hover:border-emphasis/40 hover:bg-panel sm:items-end"
              >
                <span className="text-[10px] uppercase tracking-[0.18em] text-faint">Next</span>
                <span className="text-sm text-muted transition-colors group-hover:text-emphasis">
                  {next.title} →
                </span>
              </Link>
            )}
          </nav>
        </article>

        <TableOfContents sections={sections} />
      </div>
    </div>
  );
}
