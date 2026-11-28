import DocPage from "../../components/docs/DocPage";
import { CodeBlock, Section } from "../../components/docs/Content";

const sections = [
  { id: "model", title: "The model" },
  { id: "navigation", title: "Navigation" },
  { id: "monitors", title: "Multiple monitors" },
];

export default function Workspaces() {
  return (
    <DocPage
      eyebrow="Guides"
      title="Workspaces"
      description="Workspaces are dynamic, vertical, and independent for every monitor."
      sections={sections}
    >
      <Section id="model" index="01" title="The model">
        <p>
          Workspaces are stacked vertically below each other. You do not create or destroy them by
          hand: Aether keeps one empty workspace at the end of the stack and removes empty ones
          when you leave them.
        </p>
        <p>
          Each column of windows belongs to a single workspace. Moving a column between workspaces
          keeps its windows together.
        </p>
      </Section>

      <Section id="navigation" index="02" title="Navigation">
        <p>
          Move between workspaces with the focus actions, and move columns with the matching move
          actions. These are the defaults from the shipped config.
        </p>
        <CodeBlock label="config.kdl">{`Mod+Page_Down       { focus-workspace-down; }
Mod+Page_Up         { focus-workspace-up; }
Mod+Ctrl+Page_Down  { move-column-to-workspace-down; }
Mod+Ctrl+Page_Up    { move-column-to-workspace-up; }`}</CodeBlock>
      </Section>

      <Section id="monitors" index="03" title="Multiple monitors">
        <p>
          Every monitor has its own set of workspaces. When a monitor disconnects, its workspaces
          move to another monitor where possible; when it reconnects, they move back.
        </p>
        <p>
          The Overview zooms out to show workspaces and windows together, which makes it useful for
          finding your way around a multi-monitor setup.
        </p>
      </Section>
    </DocPage>
  );
}
