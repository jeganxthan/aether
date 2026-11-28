import DocPage from "../../components/docs/DocPage";
import { FeatureGrid, Note, Section } from "../../components/docs/Content";

const sections = [
  { id: "overview", title: "Overview" },
  { id: "components", title: "Components" },
  { id: "config-and-ipc", title: "Config and IPC" },
];

const components = [
  {
    number: "01",
    title: "Compositor",
    description:
      "The Rust-based Wayland compositor built with Smithay, responsible for window management, input handling, display outputs, and the rendering foundation.",
  },
  {
    number: "02",
    title: "Shell",
    description:
      "The graphical desktop experience built with GPUI, bringing together the panel, workspace interactions, application launcher, and desktop interface.",
  },
  {
    number: "03",
    title: "Settings",
    description:
      "A native GPUI application for configuring the system, managing Wi-Fi, appearance, and other desktop and operating system preferences.",
  },
  {
    number: "04",
    title: "Services",
    description:
      "Rust-based background services that handle system functionality and communicate with applications through D-Bus, with shared infrastructure and theming.",
  },
];

export default function Architecture() {
  return (
    <DocPage
      eyebrow="Reference"
      title="Architecture"
      description="A map of the pieces that make up Aether and how they fit together."
      sections={sections}
    >
      <Section id="overview" index="01" title="Overview">
        <p>
          Aether is designed as a collection of closely integrated components, each
          responsible for a different part of the operating system experience. At its
          foundation is a Rust-based Wayland compositor built with Smithay, managing
          windows, input, and displays while providing the foundation for the shell,
          settings application, and system services.
        </p>

        <Note>
          Aether follows a modular architecture. The compositor, shell, settings, and
          system services have distinct responsibilities, but are designed to work
          together as part of a unified Linux distribution.
        </Note>
      </Section>

      <Section id="components" index="02" title="Components">
        <FeatureGrid items={components} />
      </Section>


    </DocPage>
  );
}
