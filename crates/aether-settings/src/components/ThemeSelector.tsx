import { Check } from "lucide-react";
import { themes, useTheme } from "../theme/ThemeProvider";

export default function ThemeSelector() {
  const { theme, setTheme } = useTheme();

  return (
    <div className="mt-6">
      <h2 className="text-base font-medium">Accent color</h2>
      <p className="mt-1 text-sm text-neutral-400">
        Customize the highlight color across Aether.
      </p>

      <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3">
        {Object.entries(themes).map(([key, item]) => {
          const selected = theme === key;

          return (
            <button
              key={key}
              onClick={() => setTheme(key as keyof typeof themes)}
              aria-pressed={selected}
              className={`flex items-center gap-3 rounded-xl border p-3 text-left transition-colors ${
                selected
                  ? "border-[var(--accent-border)] bg-[var(--accent-soft)]"
                  : "border-white/10 bg-white/[0.03] hover:bg-white/[0.06]"
              }`}
            >
              <span
                className="flex size-8 items-center justify-center rounded-full"
                style={{ backgroundColor: item.color }}
              >
                {selected && <Check size={16} className="text-neutral-950" />}
              </span>

              <span className="text-sm text-neutral-200">
                {item.name}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}