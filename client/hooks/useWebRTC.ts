"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { storage } from "@/lib/storage";

const WS_URL = process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:4000";

interface ServerMessage {
  type: string;
  peer_id?: string;
  sdp?: string;
  candidate?: string;
  reason?: string;
}

export interface RemoteStream {
  peerId: string;
  stream: MediaStream;
}

export function useWebRTC(roomId: string) {
  const wsRef = useRef<WebSocket | null>(null);
  const pcRef = useRef<RTCPeerConnection | null>(null);
  const localStreamRef = useRef<MediaStream | null>(null);
  const [localStream, setLocalStream] = useState<MediaStream | null>(null);
  const [remoteStreams, setRemoteStreams] = useState<RemoteStream[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Stable callback — uses refs so it never changes and the useEffect cleanup
  // doesn't fire mid-connection when localStream state updates.
  const disconnect = useCallback(() => {
    wsRef.current?.close();
    pcRef.current?.close();
    localStreamRef.current?.getTracks().forEach((t) => t.stop());
    localStreamRef.current = null;
    setConnected(false);
    setLocalStream(null);
    setRemoteStreams([]);
  }, []);

  const connect = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: true,
        audio: true,
      });
      localStreamRef.current = stream;
      setLocalStream(stream);

      const token = storage.getToken();
      const ws = new WebSocket(
        `${WS_URL}/rooms/${roomId}/join?token=${token}`
      );
      wsRef.current = ws;

      const pc = new RTCPeerConnection({
        iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
      });
      pcRef.current = pc;

      stream.getTracks().forEach((track) => pc.addTrack(track, stream));

      pc.ontrack = (event) => {
        const [remote] = event.streams;
        setRemoteStreams((prev) => {
          const exists = prev.find((r) => r.stream.id === remote.id);
          if (exists) return prev;
          return [...prev, { peerId: remote.id, stream: remote }];
        });
      };

      pc.onicecandidate = (event) => {
        if (event.candidate && ws.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              type: "ice_candidate",
              candidate: JSON.stringify(event.candidate),
            })
          );
        }
      };

      ws.onmessage = async (event) => {
        const msg: ServerMessage = JSON.parse(event.data);

        if (msg.type === "joined") {
          const offer = await pc.createOffer();
          await pc.setLocalDescription(offer);
          ws.send(JSON.stringify({ type: "offer", sdp: offer.sdp }));
        }

        if (msg.type === "answer" && msg.sdp) {
          await pc.setRemoteDescription(
            new RTCSessionDescription({ type: "answer", sdp: msg.sdp })
          );
          setConnected(true);
        }

        if (msg.type === "ice_candidate" && msg.candidate) {
          try {
            await pc.addIceCandidate(
              new RTCIceCandidate(JSON.parse(msg.candidate))
            );
          } catch {
            // ignore stale candidates
          }
        }

        if (msg.type === "error") {
          setError(msg.reason ?? "Unknown error");
        }
      };

      ws.onerror = () => setError("WebSocket error");
      ws.onclose = () => setConnected(false);
    } catch (err) {
      setError(String(err));
    }
  }, [roomId]);

  useEffect(() => {
    return () => {
      disconnect();
    };
  }, [disconnect]);

  return { localStream, remoteStreams, connected, error, connect, disconnect };
}
