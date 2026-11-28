import DocPage from "../../components/docs/DocPage";
import { FeatureGrid, Section } from "../../components/docs/Content";

const sections = [
  { id: "what-is-aether", title: "What is Aether?" },
  { id: "philosophy", title: "Design philosophy" },
  { id: "features", title: "Key features" },
  { id: "getting-started", title: "Getting started" },
];

const features = [
  {
    number: "01",
    title: "Scrollable tiling",
    description: "Windows are arranged in columns on an infinite strip. Existing windows never resize.",
  },
  {
    number: "02",
    title: "Dynamic workspaces",
    description: "Each monitor has its own workspaces, with an empty workspace always waiting below.",
  },
  {
    number: "03",
    title: "Multi-monitor",
    description: "Every monitor keeps an independent strip, including across disconnect and reconnect.",
  },
  {
    number: "04",
    title: "Rust-Powered Performance",
    description:
      "Built with Rust for memory safety, efficient resource management, and blazing-fast performance without compromising responsiveness.",
  },
  // {
  //   number: "04",
  //   title: "Live configuration",
  //   description: "A readable KDL config that reloads as you edit it, without restarting your session.",
  // },
];

export default function Introduction() {
  return (
    <DocPage
      eyebrow="Getting started"
      title="Introduction"
      description="Learn what Aether is, the ideas behind it, and how to get a session running."
      sections={sections}
    >

      <div className="border border-line bg-panel px-7 py-9 sm:px-10 sm:py-11">
        <p className="text-[10px] font-semibold uppercase tracking-[0.22em] text-emphasis">
          Introducing Aether
        </p>

        <h2 className="mt-5 max-w-xl font-display text-3xl leading-tight tracking-tight sm:text-4xl">
          A desktop that moves with you.
        </h2>

        <p className="mt-4 max-w-xl text-sm leading-7 text-muted">

          Aether is a modern Wayland compositor with dynamic tiling and floating window management.
          Arrange windows effortlessly with flexible layouts, smooth transitions, and the freedom
          to organize your desktop exactly the way you want.

        </p>
      </div>


      <Section id="what-is-aether" index="01" title="What is Aether?">
        <p>
          Aether is an independent Linux distribution in development, built with Rust
          and designed around a modern Wayland experience. It brings together a custom
          compositor, desktop shell, graphical settings, and essential system services
          into one cohesive platform.
        </p>
        <p>
          Built on the foundations of open-source Linux technologies, Aether aims to
          provide a thoughtfully integrated desktop experience. From window management
          and system configuration to service communication through D-Bus, every
          component is an opportunity to explore and build a better understanding of
          Linux from the ground up.
        </p>
      </Section>

      <Section id="philosophy" index="02" title="Design philosophy">
        <p>
          Performance, modularity, and user control are the foundations of Aether. Layouts should
          be predictable, configuration should be readable, and the compositor should never make
          you fight for screen space.
        </p>
      </Section>

      <Section id="features" index="03" title="Key features">
        <FeatureGrid items={features} />
      </Section>

      {/* <Section id="getting-started" index="04" title="Getting started">
        <p>
          Build the compositor, create a configuration file, and start a Wayland session. From
          there, the configuration guide covers the settings you are most likely to change.
        </p>
        <div className="grid gap-3 sm:grid-cols-2">
          <Link
            to="/docs/installation"
            className="group border border-line px-5 py-4 transition-colors hover:border-emphasis/40 hover:bg-panel"
          >
            <span className="text-[10px] uppercase tracking-[0.18em] text-faint">Next step</span>
            <p className="mt-1.5 text-sm text-muted group-hover:text-emphasis">Installation →</p>
          </Link>
          <Link
            to="/docs/configuration"
            className="group border border-line px-5 py-4 transition-colors hover:border-emphasis/40 hover:bg-panel"
          >
            <span className="text-[10px] uppercase tracking-[0.18em] text-faint">Reference</span>
            <p className="mt-1.5 text-sm text-muted group-hover:text-emphasis">Configuration →</p>
          </Link>
        </div>
      </Section> */}
    </DocPage>
  );
}
