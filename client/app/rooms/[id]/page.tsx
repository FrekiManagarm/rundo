"use client";

import { use, useEffect } from "react";
import { useRouter } from "next/navigation";
import { storage } from "@/lib/storage";
import { useWebRTC } from "@/hooks/useWebRTC";
import { VideoGrid } from "@/components/VideoGrid";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface PageProps {
  params: Promise<{ id: string }>;
}

export default function RoomPage({ params }: PageProps) {
  const { id: roomId } = use(params);
  const router = useRouter();
  const { localStream, remoteStreams, connected, error, connect, disconnect } =
    useWebRTC(roomId);

  useEffect(() => {
    if (!storage.getToken()) {
      router.push("/login");
    }
  }, [router]);

  function handleLeave() {
    disconnect();
    router.push("/rooms");
  }

  return (
    <main className="flex min-h-screen flex-col p-4 gap-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="text-xl font-semibold">Room</h1>
          <Badge variant={connected ? "default" : "secondary"}>
            {connected ? "Live" : "Disconnected"}
          </Badge>
        </div>
        <div className="flex gap-2">
          {!localStream ? (
            <Button onClick={connect}>Join</Button>
          ) : (
            <Button variant="destructive" onClick={disconnect}>
              Mute / Stop camera
            </Button>
          )}
          <Button variant="outline" onClick={handleLeave}>
            Leave
          </Button>
        </div>
      </div>

      {error && (
        <p className="text-sm text-destructive border border-destructive/30 rounded p-2">
          {error}
        </p>
      )}

      <div className="flex-1">
        <VideoGrid localStream={localStream} remoteStreams={remoteStreams} />
      </div>
    </main>
  );
}
