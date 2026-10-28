
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import Sidebar, {
  type SettingsPage,
} from "./components/Sidebar";
import Display from "./pages/Display";
import BluetoothPage from "./pages/Bluetooth";
import WifiPage from "./pages/Wifi";
import Appearance from "./pages/Appearance";
import Sound from "./pages/Sound";

export default function App() {
  const [activePage, setActivePage] =
    useState<SettingsPage>("General");

  const [rounded, setRounded] = useState(true);

  useEffect(() => {
    const id = window.setTimeout(() => {
      const el = document.querySelector("main");

      void fetch(
        `http://127.0.0.1:8765/state?class=${encodeURIComponent(
          el?.className ?? "none",
        )}`,
      );

      void invoke<boolean>("window_is_rounded")
        .then((value) =>
          fetch(
            `http://127.0.0.1:8765/invoke?ok=${value}`,
          ),
        )
        .catch((error) =>
          fetch(
            `http://127.0.0.1:8765/invoke?err=${encodeURIComponent(
              String(error),
            )}`,
          ),
        );
    }, 1200);

    return () => window.clearTimeout(id);
  }, []);

  useEffect(() => {
    let alive = true;
    let unlisten: UnlistenFn | undefined;

    listen<boolean>("window-rounded-changed", (event) => {
      if (alive) {
        setRounded(event.payload);
      }
    })
      .then((stop) => {
        unlisten = stop;
        return invoke<boolean>("window_is_rounded");
      })
      .then((value) => {
        if (alive) {
          setRounded(value);
        }
      })
      .catch((error) => {
        console.error(
          "Could not read window rounding state:",
          error,
        );
      });

    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  return (
    <main
      className={`flex h-full w-full overflow-hidden bg-black/30 text-neutral-100 backdrop-blur-[38px] backdrop-saturate-150 ${rounded ? "rounded-4xl" : "rounded-none"
        }`}
    >
      <Sidebar
        activePage={activePage}
        onNavigate={setActivePage}
      />

      <section className="min-h-0 min-w-0 flex-1 overflow-y-auto rounded-l-[30px] bg-[#141619]/88 p-9">
        {activePage === "Bluetooth" ? (
          <BluetoothPage />
        ) : activePage === "Network" ? (
          <WifiPage active={true} />
        ) : activePage === "Appearance" ? (
          <Appearance />
        ) : activePage === "Display" ? (
          <Display />
        ) : activePage === "Sound" ? (
          <Sound />
        ) : (
          <>
            <h1 className="text-[30px] font-semibold tracking-[-0.8px]">
              {activePage}
            </h1>

            <p className="mt-2 text-sm text-neutral-400">
              Manage your Aether desktop settings.
            </p>
          </>
        )}
      </section>
    </main>
  );
}
