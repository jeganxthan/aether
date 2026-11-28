import { useEffect, useState } from "react";
import { Link, Outlet, useLocation } from "react-router-dom";

import Sidebar from "../sidebar/Sidebar";
import { findPage } from "../../config/navigation";
import aetherLogo from "../../assets/aether.png";

export default function DocsLayout() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const { pathname } = useLocation();
  const { page } = findPage(pathname);

  useEffect(() => {
    if (!sidebarOpen) return;

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setSidebarOpen(false);
    };

    window.addEventListener("keydown", onKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [sidebarOpen]);

  return (
    <div className="min-h-screen bg-bg text-ink">
      <a
        href="#content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[60] focus:bg-panel focus:px-4 focus:py-2 focus:text-sm focus:text-ink"
      >
        Skip to content
      </a>

      <Sidebar open={sidebarOpen} onClose={() => setSidebarOpen(false)} />

      {sidebarOpen && (
        <button
          type="button"
          aria-label="Close navigation"
          onClick={() => setSidebarOpen(false)}
          className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm lg:hidden"
        />
      )}

      <div className="lg:pl-[272px]">
        <header className="sticky top-0 z-30 flex h-16 items-center gap-3 border-b border-line bg-bg/85 px-5 backdrop-blur-xl sm:px-8 lg:px-12">
          <button
            type="button"
            aria-label="Open navigation"
            aria-expanded={sidebarOpen}
            onClick={() => setSidebarOpen(true)}
            className="-ml-2 grid size-9 place-items-center text-muted transition-colors hover:text-ink lg:hidden"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <path d="M3 5.5h14M3 10h14M3 14.5h14" stroke="currentColor" strokeWidth="1.4" />
            </svg>
          </button>

          <Link
            to="/docs/introduction"
            className="inline-flex items-center gap-2 font-display text-sm font-semibold lg:hidden"
          >
            <img src={aetherLogo} alt="" className="size-5 object-contain" />
            Aether
          </Link>

          <div className="hidden items-center gap-2 text-xs text-faint sm:flex">
            <span>Docs</span>
            <span aria-hidden="true">/</span>
            <span className="text-muted">{page.title}</span>
          </div>

          <div className="ml-auto flex items-center gap-4">
            <span className="hidden font-mono text-[11px] text-faint md:inline">v0.1.0</span>
            <a
              href="https://github.com/aether-wm/aether"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 text-xs text-muted transition-colors hover:text-emphasis"
            >
              GitHub
              <span aria-hidden="true">↗</span>
            </a>
          </div>
        </header>

        <main id="content" className="min-h-[calc(100vh-4rem)]">
          <Outlet />
        </main>

        <footer className="border-t border-line px-5 py-6 text-[11px] text-faint sm:px-8 lg:px-12">
          <div className="mx-auto flex max-w-[1180px] flex-wrap items-center justify-between gap-3">
            <span>Aether documentation</span>
            <span>Built for the Wayland desktop.</span>
          </div>
        </footer>
      </div>
    </div>
  );
}
