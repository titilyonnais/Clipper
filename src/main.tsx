import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { SettingsProvider } from "@/lib/settings";
import { MainApp } from "@/main/MainApp";
import { PopupApp } from "@/popup/PopupApp";
import { ToastHost } from "@/ui/toast";
import "./styles.css";

// One bundle, two windows: the label decides which interface to show.
const isPopup = getCurrentWindow().label === "popup";

// No browser context menu (reload, inspect…) outside of editable text.
window.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest("input, textarea, .selectable")) e.preventDefault();
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <SettingsProvider>
      {isPopup ? <PopupApp /> : <MainApp />}
      <ToastHost />
    </SettingsProvider>
  </StrictMode>,
);
