import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";

export const themes = {
  violet: { name: "Violet", color: "#9b87f5" },
  blue: { name: "Blue", color: "#60a5fa" },
  green: { name: "Green", color: "#4ade80" },
  orange: { name: "Orange", color: "#fb923c" },
  rose: { name: "Rose", color: "#fb7185" },
  monochrome: { name: "Monochrome", color: "#e5e5e5" },
} as const;

export type ThemeName = keyof typeof themes;

interface ThemeContextType {
  theme: ThemeName;
  setTheme: (theme: ThemeName) => void;
}

const ThemeContext = createContext<ThemeContextType | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<ThemeName>(() => {
    try {
      const saved = localStorage.getItem("aether-theme");
      if (saved && saved in themes) return saved as ThemeName;
    } catch {
      // Use the default theme when storage is unavailable.
    }
    return "blue";
  });

  useEffect(() => {
    const color = themes[theme].color;
    const root = document.documentElement;

    root.style.setProperty("--accent", color);
    root.style.setProperty("--accent-soft", `${color}22`);
    root.style.setProperty("--accent-border", `${color}55`);

    try {
      localStorage.setItem("aether-theme", theme);
    } catch {
      // The theme still works for the current session.
    }
  }, [theme]);

  return (
    <ThemeContext.Provider
      value={{
        theme,
        setTheme: setThemeState,
      }}
    >
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);

  if (!context) {
    throw new Error("useTheme must be used inside ThemeProvider");
  }

  return context;
}