"use client";

import { useEffect, useRef } from "react";
import { Camera } from "lucide-react";

interface VideoTileProps {
  stream: MediaStream;
  muted?: boolean;
  label: string;
  isLocal?: boolean;
}

function VideoTile({ stream, muted, label, isLocal }: VideoTileProps) {
  const ref = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = ref.current;
    if (!video) return;
    video.srcObject = stream;
    video.play().catch(() => {});
  }, [stream]);

  return (
    <div className="relative rounded-xl overflow-hidden aspect-video bg-[oklch(0.11_0.007_264)] border border-white/[0.06] shadow-xl group">
      <video
        ref={ref}
        autoPlay
        playsInline
        muted={muted}
        className="w-full h-full object-cover"
      />
      <div className="absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-black/70 to-transparent pointer-events-none" />
      <span className="absolute bottom-3 left-3 text-[11px] font-mono text-white/75 tracking-widest uppercase">
        {label}
      </span>
      {isLocal && (
        <div className="absolute top-3 right-3 flex items-center gap-1.5">
          <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse-live" />
          <span className="text-[10px] font-mono text-primary/80 tracking-[0.2em] uppercase">
            live
          </span>
        </div>
      )}
    </div>
  );
}

interface VideoGridProps {
  localStream: MediaStream | null;
  remoteStreams: { peerId: string; stream: MediaStream }[];
  isScreenSharing?: boolean;
}

export function VideoGrid({ localStream, remoteStreams, isScreenSharing }: VideoGridProps) {
  const count = (localStream ? 1 : 0) + remoteStreams.length;

  if (count === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full min-h-[380px] gap-4">
        <div className="w-20 h-20 rounded-2xl border-2 border-dashed border-border/60 flex items-center justify-center">
          <Camera className="w-8 h-8 text-muted-foreground/30" />
        </div>
        <div className="text-center">
          <p className="text-sm font-semibold text-foreground/60 mb-1">
            No camera active
          </p>
          <p className="text-xs text-muted-foreground">
            Press{" "}
            <span className="text-primary font-semibold">Join</span> to connect
            your camera
          </p>
        </div>
      </div>
    );
  }

  const cols =
    count <= 1 ? "grid-cols-1" : count <= 4 ? "grid-cols-2" : "grid-cols-3";

  return (
    <div className={`grid gap-3 h-full ${cols}`}>
      {localStream && (
        <VideoTile stream={localStream} muted label={isScreenSharing ? "Screen" : "You"} isLocal />
      )}
      {remoteStreams.map(({ peerId, stream }) => (
        <VideoTile
          key={peerId}
          stream={stream}
          label={peerId.slice(0, 6).toUpperCase()}
        />
      ))}
    </div>
  );
}
