"use client";

import { use, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { storage } from "@/lib/storage";
import { roomsApi } from "@/lib/api";
import { useWebRTC } from "@/hooks/useWebRTC";
import { VideoGrid } from "@/components/VideoGrid";
import { ChatPanel } from "@/components/ChatPanel";
import { Button } from "@/components/ui/button";
import {
  ChevronLeft,
  Mic,
  MicOff,
  Monitor,
  MonitorOff,
  Video,
  VideoOff,
  Camera,
  CameraOff,
  MessageSquare,
} from "lucide-react";

interface PageProps {
  params: Promise<{ id: string }>;
}

export default function RoomPage({ params }: PageProps) {
  const { id: roomId } = use(params);
  const router = useRouter();
  const [roomError, setRoomError] = useState<string | null>(null);
  const [chatOpen, setChatOpen] = useState(false);
  const {
    localStream,
    remoteStreams,
    connected,
    isScreenSharing,
    isMuted,
    isCameraOff,
    messages,
    error,
    connect,
    disconnect,
    toggleMic,
    toggleCamera,
    sendMessage,
    startScreenShare,
    stopScreenShare,
  } = useWebRTC(roomId);

  useEffect(() => {
    if (!storage.getToken()) {
      router.push("/login");
      return;
    }
    roomsApi.get(roomId).catch(() => {
      setRoomError(
        "Room not found. It may have been deleted or the server was restarted."
      );
    });
  }, [roomId, router]);

  function handleLeave() {
    disconnect();
    router.push("/rooms");
  }

  const participantCount = (localStream ? 1 : 0) + remoteStreams.length;

  return (
    <div className="flex flex-col min-h-screen bg-background">
      <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-primary/40 to-transparent" />

      <header className="flex-shrink-0 flex items-center justify-between px-5 py-3.5 border-b border-border/50 bg-background/95 backdrop-blur-sm sticky top-0 z-10">
        <div className="flex items-center gap-3">
          <button
            onClick={handleLeave}
            className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors text-sm"
          >
            <ChevronLeft className="w-4 h-4" />
            <span className="font-black text-lg tracking-tight text-primary">
              rundo
            </span>
          </button>

          <div className="w-px h-4 bg-border" />

          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold text-foreground">Room</span>
            <div
              className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-mono font-semibold tracking-wider ${
                connected
                  ? "bg-emerald-500/15 text-emerald-400"
                  : "bg-muted text-muted-foreground"
              }`}
            >
              <span
                className={`w-1.5 h-1.5 rounded-full ${
                  connected ? "bg-emerald-400 animate-pulse-live" : "bg-muted-foreground/50"
                }`}
              />
              {connected ? "LIVE" : "OFFLINE"}
            </div>
          </div>

          {participantCount > 0 && (
            <span className="text-xs text-muted-foreground font-mono">
              {participantCount} in room
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {!localStream ? (
            <Button
              onClick={connect}
              disabled={!!roomError}
              size="sm"
              className="gap-2 font-semibold"
            >
              <Video className="w-3.5 h-3.5" />
              Join
            </Button>
          ) : (
            <>
              <Button
                variant={isMuted ? "destructive" : "outline"}
                size="sm"
                onClick={toggleMic}
                className="gap-2 border-border/60 hover:border-muted-foreground/30"
              >
                {isMuted ? (
                  <MicOff className="w-3.5 h-3.5" />
                ) : (
                  <Mic className="w-3.5 h-3.5" />
                )}
                {isMuted ? "Unmute" : "Mute"}
              </Button>
              <Button
                variant={isCameraOff ? "destructive" : "outline"}
                size="sm"
                onClick={toggleCamera}
                className="gap-2 border-border/60 hover:border-muted-foreground/30"
              >
                {isCameraOff ? (
                  <CameraOff className="w-3.5 h-3.5" />
                ) : (
                  <Camera className="w-3.5 h-3.5" />
                )}
                {isCameraOff ? "Show Camera" : "Hide Camera"}
              </Button>
              <Button
                variant={isScreenSharing ? "destructive" : "outline"}
                size="sm"
                onClick={isScreenSharing ? stopScreenShare : startScreenShare}
                className="gap-2 border-border/60 hover:border-muted-foreground/30"
              >
                {isScreenSharing ? (
                  <MonitorOff className="w-3.5 h-3.5" />
                ) : (
                  <Monitor className="w-3.5 h-3.5" />
                )}
                {isScreenSharing ? "Stop Sharing" : "Share Screen"}
              </Button>
            </>
          )}

          <Button
            variant={chatOpen ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setChatOpen((v) => !v)}
            className="gap-2 relative"
          >
            <MessageSquare className="w-3.5 h-3.5" />
            Chat
            {messages.length > 0 && !chatOpen && (
              <span className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-primary text-[9px] font-bold text-primary-foreground flex items-center justify-center">
                {messages.length > 9 ? "9+" : messages.length}
              </span>
            )}
          </Button>

          <Button
            variant="ghost"
            size="sm"
            onClick={handleLeave}
            className="text-muted-foreground hover:text-destructive hover:bg-destructive/10 gap-2"
          >
            <VideoOff className="w-3.5 h-3.5" />
            Leave
          </Button>
        </div>
      </header>

      {(roomError ?? error) && (
        <div className="mx-5 mt-4 flex-shrink-0">
          <div className="text-sm text-destructive bg-destructive/10 border border-destructive/25 rounded-xl px-4 py-3">
            {roomError ?? error}
          </div>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <div className={`flex-1 p-5 ${chatOpen ? "min-w-0" : ""}`}>
          <VideoGrid
            localStream={localStream}
            remoteStreams={remoteStreams}
            isScreenSharing={isScreenSharing}
          />
        </div>

        {chatOpen && (
          <div className="w-80 flex-shrink-0 flex flex-col" style={{ height: "calc(100vh - 57px)" }}>
            <ChatPanel messages={messages} onSend={sendMessage} />
          </div>
        )}
      </div>
    </div>
  );
}
