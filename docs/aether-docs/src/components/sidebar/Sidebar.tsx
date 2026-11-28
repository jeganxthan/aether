import { Link, useLocation } from "react-router-dom";

import { navigation } from "../../config/navigation";
import aetherLogo from "../../assets/aether.png";

type SidebarProps = {
  open: boolean;
  onClose: () => void;
};

export default function Sidebar({ open, onClose }: SidebarProps) {
  const { pathname } = useLocation();

  return (
    <aside
      className={`fixed inset-y-0 left-0 z-50 flex w-[272px] flex-col border-r border-line bg-bg transition-transform duration-300 ease-out lg:translate-x-0 ${
        open ? "translate-x-0" : "-translate-x-full"
      }`}
    >
      <div className="flex h-16 shrink-0 items-center border-b border-line px-5">
        <Link to="/docs/introduction" onClick={onClose} className="group flex items-center gap-3">
          <img
            src={aetherLogo}
            alt=""
            className="size-9 object-contain transition-opacity group-hover:opacity-70"
          />
          <span className="flex flex-col leading-none">
            <span className="font-display text-[17px] tracking-tight">AETHER</span>
            <span className="mt-1.5 text-[9px] uppercase tracking-[0.2em] text-faint">
              Documentation
            </span>
          </span>
        </Link>
      </div>

      <div className="flex shrink-0 items-center justify-between px-5 py-6">
        <span className="text-[10px] uppercase tracking-[0.18em] text-faint">Guide</span>
        <span className="border border-line px-2 py-1 font-mono text-[10px] text-muted">v0.1.0</span>
      </div>

      <nav aria-label="Documentation" className="sidebar-scroll flex-1 space-y-8 overflow-y-auto px-3 pb-8">
        {navigation.map((group) => (
          <div key={group.title}>
            <h2 className="mb-3 px-3 text-[10px] font-semibold uppercase tracking-[0.2em] text-faint">
              {group.title}
            </h2>
            <div className="flex flex-col gap-0.5">
              {group.items.map((item) => {
                const active = pathname === item.href;

                return (
                  <Link
                    key={item.href}
                    to={item.href}
                    onClick={onClose}
                    aria-current={active ? "page" : undefined}
                    className={`relative rounded-sm px-3 py-2 text-[13px] transition-colors ${
                      active
                        ? "bg-emphasis/[0.08] text-emphasis"
                        : "text-muted hover:bg-white/[0.035] hover:text-ink"
                    }`}
                  >
                    {active && (
                      <span className="absolute inset-y-1.5 left-0 w-[2px] rounded-full bg-emphasis" />
                    )}
                    {item.title}
                  </Link>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      <div className="shrink-0 border-t border-line px-5 py-4">
        <a
          href="https://github.com/jeganxthan/aether.git"
          target="_blank"
          rel="noreferrer"
          className="flex items-center justify-between text-[11px] text-muted transition-colors hover:text-emphasis"
        >
          Source on GitHub
          <span aria-hidden="true">↗</span>
        </a>
      </div>
    </aside>
  );
}
