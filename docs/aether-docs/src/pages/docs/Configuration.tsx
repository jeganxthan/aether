import DocPage from "../../components/docs/DocPage";
import { CodeBlock, Note, Section } from "../../components/docs/Content";

const sections = [
  { id: "config-file", title: "The config file" },
  { id: "layout", title: "Layout" },
  { id: "input", title: "Input" },
  { id: "keybindings", title: "Key bindings" },
  { id: "live-reload", title: "Live reload" },
];

export default function Configuration() {
  return (
    <DocPage
      eyebrow="Guides"
      title="Configuration"
      description="Aether reads a single KDL file. Edit it, save it, and the compositor picks up the change."
      sections={sections}
    >
      <Section id="config-file" index="01" title="The config file">
        <p>
          Aether looks for <code className="font-mono text-ink">~/.config/aether/config.kdl</code>{" "}
          at startup. If the file does not exist, it falls back to the system configuration in{" "}
          <code className="font-mono text-ink">/etc/aether/config.kdl</code>. You can also pass a
          different path with the <code className="font-mono text-ink">--config</code> flag.
        </p>
        <Note>
          KDL is a readable, document-oriented language. Comments start with{" "}
          <code className="font-mono text-ink">//</code>, and a node can be disabled by prefixing it
          with <code className="font-mono text-ink">/-</code>.
        </Note>
      </Section>

      <Section id="layout" index="02" title="Layout">
        <p>
          Windows are grouped into columns on a horizontal strip. The layout section controls
          gaps, column widths, focus behavior, and how borders are drawn.
        </p>
        <CodeBlock label="config.kdl">{`layout {
    gaps 16
    center-focused-column "never"

    preset-column-widths {
        proportion 0.33333
        proportion 0.5
        proportion 0.66667
    }

    default-column-width { proportion 0.5; }
}`}</CodeBlock>
      </Section>

      <Section id="input" index="03" title="Input">
        <p>
          Keyboard layouts, touchpad behavior, mouse acceleration, and trackpoint settings all live
          under the <code className="font-mono text-ink">input</code> node. Omitting a setting keeps
          the device default.
        </p>
        <CodeBlock label="config.kdl">{`input {
    keyboard {
        xkb {
            layout "us"
        }
    }

    touchpad {
        tap
        natural-scroll
    }
}`}</CodeBlock>
      </Section>

      <Section id="keybindings" index="04" title="Key bindings">
        <p>
          The <code className="font-mono text-ink">binds</code> section maps key combinations to
          actions. Use <code className="font-mono text-ink">spawn</code> to launch an application, or
          one of the built-in compositor actions to move focus and windows.
        </p>
        <CodeBlock label="config.kdl">{`binds {
    Mod+T { spawn "alacritty"; }
    Mod+D { spawn "fuzzel"; }
    Mod+Comma { focus-column-left; }
    Mod+Period { focus-column-right; }
}`}</CodeBlock>
      </Section>

      <Section id="live-reload" index="05" title="Live reload">
        <p>
          Save the file and Aether reloads it automatically. A valid config takes effect right away
          without restarting the session; if the file has an error, the previous configuration
          stays active.
        </p>
      </Section>
    </DocPage>
  );
}
