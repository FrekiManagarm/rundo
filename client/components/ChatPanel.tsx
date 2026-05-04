"use client";

import { useEffect, useRef, useState } from "react";
import { Send } from "lucide-react";
import { ChatMessage } from "@/hooks/useWebRTC";

interface ChatPanelProps {
  messages: ChatMessage[];
  onSend: (text: string) => void;
}

export function ChatPanel({ messages, onSend }: ChatPanelProps) {
  const [draft, setDraft] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  function handleSend() {
    if (!draft.trim()) return;
    onSend(draft);
    setDraft("");
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  function formatTime(ms: number) {
    return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  return (
    <div className="flex flex-col h-full bg-background border-l border-border/50">
      <div className="px-4 py-3 border-b border-border/50 flex-shrink-0">
        <p className="text-xs font-mono font-semibold tracking-[0.15em] uppercase text-muted-foreground">
          Chat
        </p>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3 min-h-0">
        {messages.length === 0 ? (
          <p className="text-xs text-muted-foreground/50 text-center pt-6">
            No messages yet
          </p>
        ) : (
          messages.map((msg) => (
            <div
              key={msg.id}
              className={`flex flex-col gap-0.5 ${msg.isLocal ? "items-end" : "items-start"}`}
            >
              {!msg.isLocal && (
                <span className="text-[10px] font-mono text-muted-foreground/60 tracking-wider px-1">
                  {msg.peerId.slice(0, 6).toUpperCase()}
                </span>
              )}
              <div
                className={`max-w-[85%] px-3 py-2 rounded-2xl text-sm leading-relaxed break-words ${
                  msg.isLocal
                    ? "bg-primary text-primary-foreground rounded-br-sm"
                    : "bg-muted text-foreground rounded-bl-sm"
                }`}
              >
                {msg.text}
              </div>
              <span className="text-[10px] text-muted-foreground/40 px-1">
                {formatTime(msg.timestampMs)}
              </span>
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>

      <div className="flex-shrink-0 px-3 py-3 border-t border-border/50">
        <div className="flex items-center gap-2 bg-muted/50 rounded-xl px-3 py-2">
          <input
            type="text"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Message…"
            className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/50"
          />
          <button
            onClick={handleSend}
            disabled={!draft.trim()}
            className="text-primary disabled:text-muted-foreground/30 transition-colors p-0.5"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
