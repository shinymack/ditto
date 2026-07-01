import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ClipboardItem {
  id: number;
  content: string;
  created_at: string;
}

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
  if (diffSec < 60) return "Just now";
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDays = Math.floor(diffHr / 24);
  return `${diffDays}d ago`;
}

function App() {
  const [history, setHistory] = useState<ClipboardItem[]>([]);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isPaused, setIsPaused] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const fetchHistory = async (searchQuery: string) => {
    try {
      const items = await invoke<ClipboardItem[]>("get_history", {
        limit: 50,
        query: searchQuery || null,
      });
      setHistory(items);
      setSelectedIndex(0);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    fetchHistory(query);
  }, [query]);

  useEffect(() => {
    const timer = setTimeout(() => {
      if (searchInputRef.current) {
        searchInputRef.current.focus();
      }
    }, 100);

    let unlisten: (() => void) | undefined;
    listen("clipboard-updated", () => {
      if (!isPaused) {
        fetchHistory(query);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      clearTimeout(timer);
      if (unlisten) unlisten();
    };
  }, [isPaused, query]);

  useEffect(() => {
    let unlistenFocus: (() => void) | undefined;

    listen("tauri://focus", () => {
      if (searchInputRef.current) {
        searchInputRef.current.focus();
        searchInputRef.current.select();
      }
    }).then((fn) => {
      unlistenFocus = fn;
    });

    return () => {
      if (unlistenFocus) unlistenFocus();
    };
  }, []);

  useEffect(() => {
    const selectedEl = document.querySelector(".selected-item");
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        try {
          await invoke("hide_window");
        } catch (err) {
          console.error(err);
        }
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


  const activeItem = history[selectedIndex];
  const isActiveImage = activeItem?.content.startsWith("data:image/png;base64,");
  const activeItemSize = activeItem
    ? new Blob([activeItem.content]).size
    : 0;

  return (
    <div className="w-screen h-screen flex glass overflow-hidden rounded-xl border border-[var(--border-card)]">
      <div className="w-[320px] border-r border-[var(--border-card)] flex flex-col bg-[rgba(10,10,12,0.6)]">
        <div className="p-4 border-b border-[var(--border-card)] relative">
          <svg
            className="absolute left-7 top-7 w-4 h-4 text-[var(--text-secondary)]"
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
          <button
            onClick={() => setIsPaused(!isPaused)}
            className="flex items-center gap-1 hover:text-[var(--text-primary)]"
          >
            <span
              className={`w-2.5 h-2.5 rounded-full ${
                isPaused ? "bg-red-500 animate-pulse" : "bg-green-500"
              }`}
            />
            {isPaused ? "Paused" : "Live"}
          </button>
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
                    ? "bg-[rgba(234,179,8,0.15)] border border-[var(--accent-primary)] selected-item"
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
            <div className="flex-1 overflow-y-auto p-6 flex items-center justify-center select-text">
              {isActiveImage ? (
                <img
                  src={activeItem.content}
                  alt="Clipboard Preview"
                  className="max-w-full max-h-full object-contain rounded-lg border border-[var(--border-card)] shadow-lg"
                />
              ) : (
                <div className="w-full h-full font-mono text-sm whitespace-pre-wrap text-[var(--text-primary)] overflow-y-auto">
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
                  {activeItemSize} B
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
                      Ctrl+L
                    </kbd>
                    <span>Clear</span>
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
  );
}

export default App;
