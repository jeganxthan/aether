import type { ReactNode } from "react";

type SectionProps = {
  id: string;
  index: string;
  title: string;
  children: ReactNode;
};

export function Section({ id, index, title, children }: SectionProps) {
  return (
    <section id={id} className="mt-16 first:mt-0">
      <div className="mb-5 flex items-baseline gap-3">
        <span className="font-mono text-[11px] text-emphasis">{index}</span>
        <h2 className="font-display text-2xl tracking-tight sm:text-3xl">{title}</h2>
      </div>
      <div className="space-y-5 text-sm leading-7 text-muted sm:text-[15px]">{children}</div>
    </section>
  );
}

export function Note({ children }: { children: ReactNode }) {
  return (
    <div className="border-l-2 border-emphasis bg-emphasis/[0.05] px-5 py-4 text-sm leading-7 text-muted">
      {children}
    </div>
  );
}

export function CodeBlock({ label, children }: { label: string; children: string }) {
  return (
    <div className="overflow-hidden border border-line bg-panel">
      <div className="flex items-center justify-between border-b border-line px-4 py-2.5">
        <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-faint">
          {label}
        </span>
        <span aria-hidden="true" className="flex gap-1.5">
          <span className="size-1.5 rounded-full bg-white/15" />
          <span className="size-1.5 rounded-full bg-white/15" />
          <span className="size-1.5 rounded-full bg-white/15" />
        </span>
      </div>
      <pre className="overflow-x-auto px-4 py-4 font-mono text-xs leading-6 text-muted sm:text-[13px]">
        <code>{children}</code>
      </pre>
    </div>
  );
}

type FeatureGridProps = {
  items: { number: string; title: string; description: string }[];
};

export function FeatureGrid({ items }: FeatureGridProps) {
  return (
    <div className="grid gap-px border border-line bg-line sm:grid-cols-2">
      {items.map((item) => (
        <div
          key={item.number}
          className="bg-bg p-6 transition-colors hover:bg-panel sm:p-7"
        >
          <span className="font-mono text-xs text-emphasis">{item.number}</span>
          <h3 className="mt-5 font-display text-lg text-ink">{item.title}</h3>
          <p className="mt-2 text-sm leading-7 text-muted">{item.description}</p>
        </div>
      ))}
    </div>
  );
}
