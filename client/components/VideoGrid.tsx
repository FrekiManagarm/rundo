"use client";

import { useEffect, useRef } from "react";

interface VideoTileProps {
  stream: MediaStream;
  muted?: boolean;
  label: string;
}

function VideoTile({ stream, muted, label }: VideoTileProps) {
  const ref = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    if (ref.current) ref.current.srcObject = stream;
  }, [stream]);

  return (
    <div className="relative rounded-lg overflow-hidden bg-muted aspect-video">
      <video
        ref={ref}
        autoPlay
        playsInline
        muted={muted}
        className="w-full h-full object-cover"
      />
      <span className="absolute bottom-2 left-2 text-xs text-white bg-black/50 px-1 rounded">
        {label}
      </span>
    </div>
  );
}

interface VideoGridProps {
  localStream: MediaStream | null;
  remoteStreams: { peerId: string; stream: MediaStream }[];
}

export function VideoGrid({ localStream, remoteStreams }: VideoGridProps) {
  const count = (localStream ? 1 : 0) + remoteStreams.length;
  const cols =
    count <= 1 ? "grid-cols-1" : count <= 4 ? "grid-cols-2" : "grid-cols-3";

  return (
    <div className={`grid gap-2 ${cols}`}>
      {localStream && (
        <VideoTile stream={localStream} muted label="You" />
      )}
      {remoteStreams.map(({ peerId, stream }) => (
        <VideoTile key={peerId} stream={stream} label={`Peer ${peerId.slice(0, 6)}`} />
      ))}
    </div>
  );
}
