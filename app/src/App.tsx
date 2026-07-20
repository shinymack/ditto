import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const currentWindow = getCurrentWindow();

interface ClipboardItem {
  id: number;
  content: string;
  created_at: string;
}

interface Config {
  max_items: number;
  ignored_apps: string[];
  escape_clears_search: boolean;
  theme: string;
  opacity: number;
  custom_accent: string;
  custom_primary: string;
  custom_secondary: string;
  persistent_window: boolean;
}

const themePresets: Record<string, { bgBase: string, bgSurface: string, borderCard: string, textPrimary: string, textSecondary: string, accentPrimary: string, accentHover: string }> = {
  dark: {
    bgBase: "#0c0c0e", bgSurface: "rgba(20, 20, 25, 0.75)", borderCard: "rgba(255, 255, 255, 0.08)",
    textPrimary: "#f3f4f6", textSecondary: "#9ca3af", accentPrimary: "#eab308", accentHover: "#ca8a04"
  },
  cyan: {
    bgBase: "#0c0c0e", bgSurface: "rgba(20, 20, 25, 0.75)", borderCard: "rgba(255, 255, 255, 0.08)",
    textPrimary: "#f3f4f6", textSecondary: "#9ca3af", accentPrimary: "#06b6d4", accentHover: "#0891b2"
  },
  emerald: {
    bgBase: "#0c0c0e", bgSurface: "rgba(20, 20, 25, 0.75)", borderCard: "rgba(255, 255, 255, 0.08)",
    textPrimary: "#f3f4f6", textSecondary: "#9ca3af", accentPrimary: "#10b981", accentHover: "#059669"
  },
  amber: {
    bgBase: "#0c0c0e", bgSurface: "rgba(20, 20, 25, 0.75)", borderCard: "rgba(255, 255, 255, 0.08)",
    textPrimary: "#f3f4f6", textSecondary: "#9ca3af", accentPrimary: "#f59e0b", accentHover: "#d97706"
  },
  rose: {
    bgBase: "#0c0c0e", bgSurface: "rgba(20, 20, 25, 0.75)", borderCard: "rgba(255, 255, 255, 0.08)",
    textPrimary: "#f3f4f6", textSecondary: "#9ca3af", accentPrimary: "#f43f5e", accentHover: "#e11d48"
  },
  "light-pure": {
    bgBase: "#f9fafb", bgSurface: "rgba(255, 255, 255, 0.85)", borderCard: "rgba(0, 0, 0, 0.08)",
    textPrimary: "#111827", textSecondary: "#4b5563", accentPrimary: "#eab308", accentHover: "#ca8a04"
  },
  "light-nordic": {
    bgBase: "#f3f4f6", bgSurface: "rgba(243, 244, 246, 0.85)", borderCard: "rgba(0, 0, 0, 0.06)",
    textPrimary: "#1f2937", textSecondary: "#4b5563", accentPrimary: "#3b82f6", accentHover: "#2563eb"
  },
  custom: {
    bgBase: "#0c0c0e", bgSurface: "rgba(20, 20, 25, 0.75)", borderCard: "rgba(255, 255, 255, 0.08)",
    textPrimary: "#f3f4f6", textSecondary: "#9ca3af", accentPrimary: "#eab308", accentHover: "#ca8a04"
  }
};

const applyConfigTheme = (cfg: Config) => {
  const root = document.documentElement;
  const preset = themePresets[cfg.theme] || themePresets.dark;
  
  root.setAttribute("data-theme", cfg.theme);
  root.style.setProperty("--bg-base", preset.bgBase);
  
  const opacityVal = (cfg.opacity ?? 75) / 100;
  const cleanSurface = preset.bgSurface.replace(/[\d.]+\)$/, `${opacityVal})`);
  root.style.setProperty("--bg-surface", cleanSurface);
  root.style.setProperty("--border-card", preset.borderCard);
  
  if (cfg.theme === "custom") {
    root.style.setProperty("--accent-primary", cfg.custom_accent || "#eab308");
    root.style.setProperty("--accent-hover", cfg.custom_accent || "#ca8a04");
    root.style.setProperty("--text-primary", cfg.custom_primary || "#f3f4f6");
    root.style.setProperty("--text-secondary", cfg.custom_secondary || "#9ca3af");
  } else {
    root.style.setProperty("--accent-primary", preset.accentPrimary);
    root.style.setProperty("--accent-hover", preset.accentHover);
    root.style.setProperty("--text-primary", preset.textPrimary);
    root.style.setProperty("--text-secondary", preset.textSecondary);
  }
};

function getSimpleHash(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash).toString(16).padStart(8, "0");
}

function getRelativeTime(timestampStr: string): string {
  const utcStr = timestampStr.replace(" ", "T") + "Z";
  const date = new Date(utcStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  if (isNaN(diffMs) || diffMs < 0) {
    return "Just now";
  }
  const diffSec = Math.floor(diffMs / 1000);
  if (diffSec < 10) return "Just now";
  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDays = Math.floor(diffHr / 24);
  return `${diffDays}d ago`;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function SettingsView() {
  const [config, setConfig] = useState<Config | null>(null);
  const [ignoredAppsText, setIgnoredAppsText] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    const loadConfig = async () => {
      try {
        const cfg = await invoke<Config>("get_config");
        setConfig(cfg);
        setIgnoredAppsText(cfg.ignored_apps.join("\n"));
        applyConfigTheme(cfg);
      } catch (err) {
        console.error("Failed to load config:", err);
      }
    };
    loadConfig();
  }, []);

  const handleHeaderMouseDown = async (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (
      e.button === 0 &&
      target.tagName !== "BUTTON" &&
      target.tagName !== "INPUT" &&
      target.tagName !== "SELECT" &&
      target.tagName !== "TEXTAREA" &&
      !target.closest("button")
    ) {
      try {
        await currentWindow.startDragging();
      } catch (err) {
        console.error(err);
      }
    }
  };

  const handleSave = async () => {
    if (!config) return;
    setIsSaving(true);
    try {
      const parsedApps = ignoredAppsText
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      
      const newConfig = {
        ...config,
        ignored_apps: parsedApps,
      };
      
      await invoke("save_config", { config: newConfig });
      await emit("config-updated", newConfig);
      setConfig(newConfig);
      await currentWindow.hide();
    } catch (err) {
      console.error("Failed to save config:", err);
    } finally {
      setIsSaving(false);
    }
  };

  if (!config) {
    return (
      <div className="w-screen h-screen flex items-center justify-center bg-transparent text-[var(--text-secondary)] text-sm">
        Loading settings...
      </div>
    );
  }

  return (
    <div className="w-screen h-screen flex flex-col glass p-6 rounded-xl border border-[var(--border-card)] text-[var(--text-primary)] font-sans select-none overflow-hidden">
      <div 
        onMouseDown={handleHeaderMouseDown}
        className="flex items-center justify-between border-b border-[rgba(255,255,255,0.06)] pb-4 mb-4 cursor-move"
      >
        <h1 className="text-base font-medium flex items-center gap-2">
          <img src="/logo.png" className="w-4 h-4 object-contain rounded" alt="Ditto Logo" />
          Ditto Settings
        </h1>
        <div className="flex items-center gap-2">
          <span className="text-[9px] text-[var(--text-secondary)] bg-[rgba(255,255,255,0.04)] px-2 py-0.5 rounded border border-[rgba(255,255,255,0.06)] font-mono">
            config.json
          </span>
          <button
            onClick={() => currentWindow.hide()}
            className="p-1 hover:bg-[rgba(255,255,255,0.05)] rounded-lg transition-colors text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
            title="Close"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      <div className="flex-1 space-y-5 overflow-y-auto pr-3">
        {/* 1. Persistent Window Mode (Moved to First Setting) */}
        <div className="flex items-center justify-between border-b border-[rgba(255,255,255,0.04)] pb-4">
          <div className="space-y-1 pr-4">
            <label className="text-xs font-semibold text-[var(--text-primary)]">Persistent Window Mode</label>
            <p className="text-[10px] text-[var(--text-secondary)]">If enabled, the window remains open when you click outside. It also displays a custom header with a title and close button.</p>
          </div>
          <button
            type="button"
            onClick={async () => {
              const newCfg = { ...config, persistent_window: !config.persistent_window };
              setConfig(newCfg);
              applyConfigTheme(newCfg);
              await emit("config-updated", newCfg);
            }}
            className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none mr-4 ${
              config.persistent_window ? "bg-[var(--accent-primary)]" : "bg-[rgba(255,255,255,0.15)]"
            }`}
          >
            <span
              className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                config.persistent_window ? "translate-x-4" : "translate-x-0"
              }`}
            />
          </button>
        </div>

        {/* 2. Escape Clears Search Query (Toggle) */}
        <div className="flex items-center justify-between border-b border-[rgba(255,255,255,0.04)] pb-4">
          <div className="space-y-1 pr-4">
            <label className="text-xs font-semibold text-[var(--text-primary)]">Escape Clears Search Query</label>
            <p className="text-[10px] text-[var(--text-secondary)]">Pressing Escape clears the search bar. If disabled, it only hides the window.</p>
          </div>
          <button
            type="button"
            onClick={async () => {
              const newCfg = { ...config, escape_clears_search: !config.escape_clears_search };
              setConfig(newCfg);
              applyConfigTheme(newCfg);
              await emit("config-updated", newCfg);
            }}
            className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none mr-4 ${
              config.escape_clears_search ? "bg-[var(--accent-primary)]" : "bg-[rgba(255,255,255,0.15)]"
            }`}
          >
            <span
              className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                config.escape_clears_search ? "translate-x-4" : "translate-x-0"
              }`}
            />
          </button>
        </div>

        {/* 3. History Size Limit */}
        <div className="space-y-3">
          <label className="text-xs text-[var(--text-secondary)] font-medium">History Size Limit</label>
          <input
            type="number"
            value={config.max_items}
            onChange={async (e) => {
              const newCfg = { ...config, max_items: parseInt(e.target.value) || 0 };
              setConfig(newCfg);
              applyConfigTheme(newCfg);
              await emit("config-updated", newCfg);
            }}
            className="w-full bg-[rgba(255,255,255,0.02)] border border-[rgba(255,255,255,0.06)] rounded-lg py-2 px-3 text-sm focus:outline-none focus:border-[var(--accent-primary)] font-mono text-[var(--text-primary)]"
            min={1}
            max={5000}
          />
          <p className="text-[10px] text-[var(--text-secondary)]">Maximum number of clipboard items retained in the SQLite database.</p>
        </div>

        {/* 4. Window Opacity */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <label className="text-xs text-[var(--text-secondary)] font-medium">Window Opacity</label>
            <span className="text-xs font-mono text-[var(--accent-primary)]">{config.opacity}%</span>
          </div>
          <input
            type="range"
            min="20"
            max="100"
            value={config.opacity}
            onChange={async (e) => {
              const val = parseInt(e.target.value) || 75;
              const newCfg = { ...config, opacity: val };
              setConfig(newCfg);
              applyConfigTheme(newCfg);
              await emit("config-updated", newCfg);
            }}
            className="w-full accent-[var(--accent-primary)] cursor-pointer"
          />
          <p className="text-[10px] text-[var(--text-secondary)]">Set background transparency level of the clipboard window.</p>
        </div>

        {/* 5. Visual Theme Accent */}
        <div className="space-y-3">
          <label className="text-xs text-[var(--text-secondary)] font-medium">Visual Theme Accent</label>
          <select
            value={config.theme}
            onChange={async (e) => {
              const themeVal = e.target.value;
              const newCfg = { ...config, theme: themeVal };
              setConfig(newCfg);
              applyConfigTheme(newCfg);
              await emit("config-updated", newCfg);
            }}
            className="w-full appearance-none bg-[var(--bg-base)] border border-[var(--border-card)] rounded-lg py-2 px-3 text-sm focus:outline-none focus:border-[var(--accent-primary)] text-[var(--text-primary)] cursor-pointer pr-10"
            style={{
              backgroundImage: `url("data:image/svg+xml;charset=utf-8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2020%2020%22%20fill%3D%22none%22%3E%3Cpath%20d%3D%22M7%209l3%203%203-3%22%20stroke%3D%22%239ca3af%22%20stroke-width%3D%221.5%22%20stroke-linecap%3D%22round%22%20stroke-linejoin%3D%22round%22%2F%3E%3C%2Fsvg%3E")`,
              backgroundPosition: "right 0.5rem center",
              backgroundSize: "1.25rem 1.25rem",
              backgroundRepeat: "no-repeat",
            }}
          >
            <option value="dark" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Ditto Yellow (Default)</option>
            <option value="cyan" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Electric Cyan</option>
            <option value="emerald" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Mint Emerald</option>
            <option value="amber" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Warm Amber</option>
            <option value="rose" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Soft Rose</option>
            <option value="light-pure" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Light Pure White</option>
            <option value="light-nordic" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Light Nordic Sand</option>
            <option value="custom" className="bg-[var(--bg-base)] text-[var(--text-primary)]">Custom Accent & Colors</option>
          </select>
          <p className="text-[10px] text-[var(--text-secondary)]">Choose the application color theme accent.</p>
        </div>

        {/* 6. Custom Colors Picker Panel */}
        {config.theme === "custom" && (
          <div className="grid grid-cols-3 gap-3 p-3 bg-[rgba(128,128,128,0.03)] border border-[var(--border-card)] rounded-lg">
            <div className="space-y-1">
              <label className="text-[10px] text-[var(--text-secondary)] font-medium">Accent</label>
              <div className="flex items-center gap-1.5 bg-[var(--bg-base)] border border-[var(--border-card)] rounded-md px-1.5 py-1">
                <input
                  type="color"
                  value={config.custom_accent}
                  onChange={async (e) => {
                    const newCfg = { ...config, custom_accent: e.target.value };
                    setConfig(newCfg);
                    applyConfigTheme(newCfg);
                    await emit("config-updated", newCfg);
                  }}
                  className="w-5 h-5 border-0 bg-transparent cursor-pointer rounded"
                />
                <span className="text-[9px] font-mono">{config.custom_accent}</span>
              </div>
            </div>
            <div className="space-y-1">
              <label className="text-[10px] text-[var(--text-secondary)] font-medium">Primary Text</label>
              <div className="flex items-center gap-1.5 bg-[var(--bg-base)] border border-[var(--border-card)] rounded-md px-1.5 py-1">
                <input
                  type="color"
                  value={config.custom_primary}
                  onChange={async (e) => {
                    const newCfg = { ...config, custom_primary: e.target.value };
                    setConfig(newCfg);
                    applyConfigTheme(newCfg);
                    await emit("config-updated", newCfg);
                  }}
                  className="w-5 h-5 border-0 bg-transparent cursor-pointer rounded"
                />
                <span className="text-[9px] font-mono">{config.custom_primary}</span>
              </div>
            </div>
            <div className="space-y-1">
              <label className="text-[10px] text-[var(--text-secondary)] font-medium">Secondary Text</label>
              <div className="flex items-center gap-1.5 bg-[var(--bg-base)] border border-[var(--border-card)] rounded-md px-1.5 py-1">
                <input
                  type="color"
                  value={config.custom_secondary}
                  onChange={async (e) => {
                    const newCfg = { ...config, custom_secondary: e.target.value };
                    setConfig(newCfg);
                    applyConfigTheme(newCfg);
                    await emit("config-updated", newCfg);
                  }}
                  className="w-5 h-5 border-0 bg-transparent cursor-pointer rounded"
                />
                <span className="text-[9px] font-mono">{config.custom_secondary}</span>
              </div>
            </div>
          </div>
        )}

        {/* 7. Ignored Applications */}
        <div className="space-y-3">
          <label className="text-xs text-[var(--text-secondary)] font-medium">Ignored Applications (Keywords)</label>
          <textarea
            value={ignoredAppsText}
            onChange={(e) => setIgnoredAppsText(e.target.value)}
            placeholder="e.g. 1password&#10;bitwarden&#10;keepassxc"
            className="w-full h-24 bg-[rgba(255,255,255,0.02)] border border-[rgba(255,255,255,0.06)] rounded-lg py-2 px-3 text-sm focus:outline-none focus:border-[var(--accent-primary)] font-mono text-[var(--text-primary)] resize-none"
          />
          <p className="text-[10px] text-[var(--text-secondary)]">Ignore clipboard entries containing these keywords (case-insensitive). One keyword per line.</p>
        </div>
      </div>

      <div className="border-t border-[rgba(255,255,255,0.06)] pt-4 mt-4 flex items-center justify-end gap-3">
        <button
          onClick={() => currentWindow.hide()}
          className="px-4 py-2 rounded-lg text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[rgba(255,255,255,0.02)] transition-colors"
        >
          Cancel
        </button>
        <button
          onClick={handleSave}
          disabled={isSaving}
          className="px-4 py-2 bg-[var(--accent-primary)] text-black rounded-lg text-sm font-medium hover:opacity-90 disabled:opacity-50 transition-colors"
        >
          {isSaving ? "Saving..." : "Save Settings"}
        </button>
      </div>
    </div>
  );
}

function App() {
  const [config, setConfig] = useState<Config | null>(null);

  // Sync theme from backend config on mount
  useEffect(() => {
    const loadTheme = async () => {
      try {
        const cfg = await invoke<Config>("get_config");
        setConfig(cfg);
        applyConfigTheme(cfg);
      } catch (err) {
        console.error(err);
      }
    };
    loadTheme();

    // Listen for real-time config updates from settings window
    let unlistenConfig: (() => void) | undefined;
    listen<Config>("config-updated", (event) => {
      setConfig(event.payload);
      applyConfigTheme(event.payload);
    }).then((fn) => {
      unlistenConfig = fn;
    });

    return () => {
      if (unlistenConfig) unlistenConfig();
    };
  }, []);

  if (currentWindow.label === "settings") {
    return <SettingsView />;
  }

  const [history, setHistory] = useState<ClipboardItem[]>([]);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isPaused, setIsPaused] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const queryRef = useRef(query);
  const isPausedRef = useRef(isPaused);
  const configRef = useRef<Config | null>(null);

  useEffect(() => {
    queryRef.current = query;
  }, [query]);

  useEffect(() => {
    isPausedRef.current = isPaused;
  }, [isPaused]);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  const lastRequestIdRef = useRef(0);

  const fetchHistory = async (searchQuery: string) => {
    const requestId = ++lastRequestIdRef.current;
    const limitVal = configRef.current ? configRef.current.max_items : 50;
    try {
      const items = await invoke<ClipboardItem[]>("get_history", {
        limit: limitVal,
        query: searchQuery || null,
      });
      if (requestId === lastRequestIdRef.current) {
        setHistory(items);
      }
    } catch (err) {
      if (requestId === lastRequestIdRef.current) {
        console.error("Failed to fetch history:", err);
      }
    }
  };

  const openSettings = async () => {
    try {
      const settingsWin = await WebviewWindow.getByLabel("settings");
      if (settingsWin) {
        await settingsWin.show();
        await settingsWin.setFocus();
      }
    } catch (err) {
      console.error("Failed to open settings window:", err);
    }
  };

  // Fetch history when query changes
  useEffect(() => {
    fetchHistory(query);
    setSelectedIndex(0);
  }, [query]);

  // Set up listeners once on mount
  useEffect(() => {
    // Focus search input on mount
    const timer = setTimeout(() => {
      if (searchInputRef.current) {
        searchInputRef.current.focus();
      }
    }, 150);

    let unlistenClipboard: (() => void) | undefined;
    let unlistenFocus: (() => void) | undefined;

    // Clipboard updates
    listen("clipboard-updated", () => {
      if (!isPausedRef.current) {
        fetchHistory(queryRef.current);
      }
    }).then((fn) => {
      unlistenClipboard = fn;
    });

    // Window focus: refocus, position cursor at the end, and reload theme config
    listen("tauri://focus", async () => {
      if (searchInputRef.current) {
        searchInputRef.current.focus();
        const len = searchInputRef.current.value.length;
        searchInputRef.current.setSelectionRange(len, len);
      }
      try {
        const cfg = await invoke<Config>("get_config");
        applyConfigTheme(cfg);
      } catch (err) {
        console.error(err);
      }
    }).then((fn) => {
      unlistenFocus = fn;
    });

    return () => {
      clearTimeout(timer);
      if (unlistenClipboard) unlistenClipboard();
      if (unlistenFocus) unlistenFocus();
    };
  }, []);

  // Sync pause state from backend on mount & listen to changes
  useEffect(() => {
    const syncPauseState = async () => {
      try {
        const paused = await invoke<boolean>("is_paused");
        setIsPaused(paused);
      } catch (err) {
        console.error(err);
      }
    };
    syncPauseState();

    let unlistenPause: (() => void) | undefined;
    listen<boolean>("pause-updated", (event) => {
      setIsPaused(event.payload);
    }).then((fn) => {
      unlistenPause = fn;
    });

    return () => {
      if (unlistenPause) unlistenPause();
    };
  }, []);

  // Keyboard navigation and actions
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        try {
          const cfg = await invoke<Config>("get_config");
          if (cfg.escape_clears_search) {
            setQuery("");
            setSelectedIndex(0);
          }
        } catch (err) {
          console.error(err);
        }
        try {
          await invoke("hide_window");
        } catch (err) {
          console.error(err);
        }
        return;
      }

      if (e.ctrlKey && e.key === ",") {
        e.preventDefault();
        openSettings();
        return;
      }

      if (e.ctrlKey && e.key === "l") {
        e.preventDefault();
        try {
          await invoke("clear_history");
          fetchHistory(query);
        } catch (err) {
          console.error(err);
        }
        return;
      }

      if (e.altKey) {
        const num = parseInt(e.key);
        if (num >= 1 && num <= 9 && history[num - 1]) {
          e.preventDefault();
          setQuery("");
          setSelectedIndex(0);
          try {
            await invoke("select_item", { content: history[num - 1].content });
          } catch (err) {
            console.error(err);
          }
          return;
        }
      }

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((prev) => (prev < history.length - 1 ? prev + 1 : prev));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : 0));
      } else if (e.key === "Enter") {
        if (history[selectedIndex]) {
          e.preventDefault();
          setQuery("");
          setSelectedIndex(0);
          try {
            await invoke("select_item", { content: history[selectedIndex].content });
          } catch (err) {
            console.error(err);
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [history, selectedIndex, query]);

  // Keep selected item visible in scroll view
  useEffect(() => {
    const selectedEl = document.querySelector(".selected-item");
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  const handleHeaderMouseDown = async (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (
      e.button === 0 &&
      target.tagName !== "BUTTON" &&
      target.tagName !== "INPUT" &&
      target.tagName !== "SELECT" &&
      target.tagName !== "TEXTAREA" &&
      !target.closest("button")
    ) {
      try {
        await currentWindow.startDragging();
      } catch (err) {
        console.error(err);
      }
    }
  };

  const activeItem = history[selectedIndex];
  const isActiveImage = activeItem?.content.startsWith("data:image/png;base64,");
  const activeItemSize = activeItem
    ? new Blob([activeItem.content]).size
    : 0;

  return (
    <div className="w-screen h-screen flex flex-col glass overflow-hidden rounded-xl border border-[var(--border-card)]">
      {config?.persistent_window && (
        <div 
          onMouseDown={handleHeaderMouseDown}
          className="h-10 border-b border-[var(--border-card)] flex items-center justify-between px-4 bg-[rgba(10,10,12,0.85)] cursor-move select-none shrink-0"
        >
          <div className="flex items-center gap-2">
            <img src="/logo.png" className="w-4 h-4 object-contain rounded" alt="Ditto Logo" />
            <span className="text-xs font-semibold tracking-wider text-[var(--text-primary)]">DITTO</span>
          </div>
          <button
            onClick={() => currentWindow.hide()}
            className="p-1 hover:bg-[rgba(255,255,255,0.05)] rounded-lg transition-colors text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
            title="Hide Window"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      )}
      
      <div className="flex-1 flex overflow-hidden">
        <div className="w-[320px] border-r border-[var(--border-card)] flex flex-col bg-[rgba(10,10,12,0.6)]">
        <div 
          onMouseDown={handleHeaderMouseDown}
          className="p-4 border-b border-[var(--border-card)] relative cursor-move flex items-center"
        >
          <svg
            className="absolute left-7 w-4 h-4 text-[var(--text-secondary)] pointer-events-none"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            ref={searchInputRef}
            type="text"
            placeholder="Browse clipboard history..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="w-full bg-[rgba(255,255,255,0.03)] border border-[var(--border-card)] rounded-lg py-2 px-3 pl-9 text-sm focus:outline-none focus:border-[var(--accent-primary)] text-[var(--text-primary)] font-sans"
          />
        </div>

        <div className="px-4 py-2 border-b border-[var(--border-card)] flex items-center justify-between text-xs text-[var(--text-secondary)]">
          <span>{history.length} Items</span>
          <div className="flex items-center gap-3">
            <button
              onClick={() => invoke("set_paused", { paused: !isPaused })}
              className="flex items-center gap-1 hover:text-[var(--text-primary)]"
            >
              <span
                className={`w-2.5 h-2.5 rounded-full ${
                  isPaused ? "bg-red-500 animate-pulse" : "bg-green-500"
                }`}
              />
              {isPaused ? "Paused" : "Live"}
            </button>
            <button
              onClick={openSettings}
              className="hover:text-[var(--text-primary)] transition-colors flex items-center"
              title="Settings (Ctrl+,)"
            >
              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {history.map((item, index) => {
            const isSelected = index === selectedIndex;
            const isImage = item.content.startsWith("data:image/png;base64,");
            const singleLine = item.content.replace(/\s+/g, " ");
            const preview = isImage
              ? "Image Clip"
              : singleLine.length > 35
              ? singleLine.substring(0, 35) + "..."
              : singleLine;

            return (
              <div
                key={item.id}
                onClick={() => setSelectedIndex(index)}
                onDoubleClick={async () => {
                  try {
                    await invoke("select_item", { content: item.content });
                  } catch (err) {
                    console.error(err);
                  }
                }}
                className={`p-3 rounded-lg cursor-pointer flex items-center justify-between transition-colors ${
                  isSelected
                    ? "bg-[rgba(255,255,255,0.05)] border border-[var(--accent-primary)] selected-item"
                    : "hover:bg-[rgba(255,255,255,0.02)] border border-transparent"
                }`}
              >
                <div className="flex items-center gap-3 overflow-hidden">
                  <div className="text-secondary select-none font-mono text-sm flex items-center justify-center">
                    {isImage ? (
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                      </svg>
                    ) : (
                      "T"
                    )}
                  </div>
                  <div className="flex flex-col overflow-hidden">
                    <span className="text-sm font-sans text-[var(--text-primary)] truncate">
                      {preview}
                    </span>
                    <span className="text-[10px] text-[var(--text-secondary)] font-mono mt-0.5">
                      {getRelativeTime(item.created_at)}
                    </span>
                  </div>
                </div>
                {index < 9 && (
                  <span className="text-[10px] text-[var(--text-secondary)] font-mono border border-[var(--border-card)] px-1 rounded select-none">
                    ⌥{index + 1}
                  </span>
                )}
              </div>
            );
          })}
        </div>
      </div>

      <div className="flex-1 flex flex-col bg-[rgba(12,12,14,0.4)]">
        {activeItem ? (
          <>
            <div className={`flex-1 overflow-y-auto p-6 flex select-text ${isActiveImage ? "items-center justify-center" : "flex-col items-start justify-start"}`}>
              {isActiveImage ? (
                <img
                  src={activeItem.content}
                  alt="Clipboard Preview"
                  className="max-w-full max-h-full object-contain rounded-lg border border-[var(--border-card)] shadow-lg"
                />
              ) : (
                <div className="w-full h-full font-mono text-sm whitespace-pre-wrap break-all text-[var(--text-primary)] overflow-y-auto pr-4">
                  {activeItem.content}
                </div>
              )}
            </div>

            <div className="border-t border-[var(--border-card)] p-6 bg-[#0f0f12] text-xs text-[var(--text-secondary)] space-y-3">
              <div className="grid grid-cols-[100px_1fr] gap-2">
                <span>Mime</span>
                <span className="font-mono text-[var(--text-primary)]">
                  {isActiveImage ? "image/png" : "text/plain"}
                </span>

                <span>Size</span>
                <span className="font-mono text-[var(--text-primary)]">
                  {formatSize(activeItemSize)}
                </span>

                <span>Copied at</span>
                <span className="font-mono text-[var(--text-primary)]">
                  {activeItem.created_at}
                </span>

                <span>Checksum</span>
                <span className="font-mono text-[var(--text-primary)]">
                  {getSimpleHash(activeItem.content)}
                </span>
              </div>

              <div className="flex items-center justify-between border-t border-[var(--border-card)] pt-3 text-[10px]">
                <div className="flex gap-4">
                  <span className="flex items-center gap-1.5">
                    <kbd className="bg-[rgba(255,255,255,0.05)] px-1.5 py-0.5 rounded border border-[var(--border-card)] font-mono text-[9px]">
                      Enter
                    </kbd>
                    <span>Copy</span>
                  </span>
                  <span className="flex items-center gap-1.5">
                    <kbd className="bg-[rgba(255,255,255,0.05)] px-1.5 py-0.5 rounded border border-[var(--border-card)] font-mono text-[9px]">
                      Esc
                    </kbd>
                    <span>Hide</span>
                  </span>
                  <span className="flex items-center gap-1.5">
                    <kbd className="bg-[rgba(255,255,255,0.05)] px-1.5 py-0.5 rounded border border-[var(--border-card)] font-mono text-[9px]">
                      Ctrl+,
                    </kbd>
                    <span>Settings</span>
                  </span>
                </div>
              </div>
            </div>
          </>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center text-sm text-[var(--text-secondary)]">
            No clipboard items found
          </div>
        )}
      </div>
    </div>
  </div>
);
}

export default App;
