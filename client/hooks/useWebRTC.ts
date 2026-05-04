"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { iceConfigApi } from "@/lib/api";
import { storage } from "@/lib/storage";

const WS_URL = process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:4000";

const FALLBACK_ICE_SERVERS: RTCIceServer[] = [
  { urls: "stun:stun.l.google.com:19302" },
];

export interface ChatMessage {
  id: string;
  peerId: string;
  text: string;
  timestampMs: number;
  isLocal: boolean;
}

export interface RemoteStream {
  peerId: string;
  stream: MediaStream;
}

export function useWebRTC(roomId: string) {
  const wsRef = useRef<WebSocket | null>(null);
  const localStreamRef = useRef<MediaStream | null>(null);
  const cameraTrackRef = useRef<MediaStreamTrack | null>(null);
  const screenTrackRef = useRef<MediaStreamTrack | null>(null);
  // Single RTCPeerConnection to the SFU server
  const pcRef = useRef<RTCPeerConnection | null>(null);
  const remoteStreamsMapRef = useRef<Map<string, MediaStream>>(new Map());
  const iceServersRef = useRef<RTCIceServer[]>(FALLBACK_ICE_SERVERS);
  const localPeerIdRef = useRef<string | null>(null);
  const pendingRemoteCandidates = useRef<RTCIceCandidateInit[]>([]);

  const [localStream, setLocalStream] = useState<MediaStream | null>(null);
  const [remoteStreams, setRemoteStreams] = useState<RemoteStream[]>([]);
  const [connected, setConnected] = useState(false);
  const [isScreenSharing, setIsScreenSharing] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isCameraOff, setIsCameraOff] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [error, setError] = useState<string | null>(null);

  const syncRemoteStreams = useCallback(() => {
    setRemoteStreams(
      Array.from(remoteStreamsMapRef.current.entries()).map(([peerId, stream]) => ({
        peerId,
        stream,
      }))
    );
  }, []);

  // Handle incoming SDP offer from server (initial or renegotiation)
  const handleOffer = useCallback(async (sdp: string) => {
    const pc = pcRef.current;
    const ws = wsRef.current;
    if (!pc || !ws) return;

    try {
      await pc.setRemoteDescription({ type: "offer", sdp });
      const buffered = pendingRemoteCandidates.current.splice(0);
      for (const c of buffered) {
        pc.addIceCandidate(c).catch(() => {});
      }
      const answer = await pc.createAnswer();
      await pc.setLocalDescription(answer);
      ws.send(JSON.stringify({ type: "answer", sdp: answer.sdp }));
    } catch (err) {
      console.error("[SFU] handleOffer failed:", err, "\nSDP:", sdp);
      setError(`SDP negotiation failed: ${err}`);
    }
  }, []);

  const stopScreenShare = useCallback(() => {
    const screenTrack = screenTrackRef.current;
    if (!screenTrack) return;

    screenTrack.onended = null;
    screenTrack.stop();
    screenTrackRef.current = null;

    const stream = localStreamRef.current;
    if (stream) {
      stream.getVideoTracks().forEach((t) => stream.removeTrack(t));

      const cameraTrack = cameraTrackRef.current;
      if (cameraTrack && cameraTrack.readyState === "live") {
        stream.addTrack(cameraTrack);
        const sender = pcRef.current?.getSenders().find((s) => s.track?.kind === "video");
        if (sender) sender.replaceTrack(cameraTrack);
      }

      setLocalStream(new MediaStream(stream.getTracks()));
    }

    setIsScreenSharing(false);
  }, []);

  const startScreenShare = useCallback(async () => {
    if (!localStreamRef.current || isScreenSharing) return;

    try {
      const screenStream = await navigator.mediaDevices.getDisplayMedia({
        video: true,
        audio: false,
      });
      const screenTrack = screenStream.getVideoTracks()[0];
      if (!screenTrack) return;

      const stream = localStreamRef.current;
      const currentVideoTrack = stream.getVideoTracks()[0];
      if (currentVideoTrack) {
        cameraTrackRef.current = currentVideoTrack;
        stream.removeTrack(currentVideoTrack);
      }

      stream.addTrack(screenTrack);
      screenTrackRef.current = screenTrack;

      const sender = pcRef.current?.getSenders().find((s) => s.track?.kind === "video");
      if (sender) sender.replaceTrack(screenTrack);

      setLocalStream(new MediaStream(stream.getTracks()));
      setIsScreenSharing(true);

      screenTrack.onended = () => stopScreenShare();
    } catch (err) {
      if ((err as DOMException).name !== "NotAllowedError") {
        setError(String(err));
      }
    }
  }, [isScreenSharing, stopScreenShare]);

  const toggleMic = useCallback(() => {
    const stream = localStreamRef.current;
    if (!stream) return;
    const newMuted = !isMuted;
    stream.getAudioTracks().forEach((t) => { t.enabled = !newMuted; });
    setIsMuted(newMuted);
  }, [isMuted]);

  const toggleCamera = useCallback(() => {
    const stream = localStreamRef.current;
    if (!stream) return;
    const newCameraOff = !isCameraOff;
    stream.getVideoTracks().forEach((t) => { t.enabled = !newCameraOff; });
    setIsCameraOff(newCameraOff);
  }, [isCameraOff]);

  const sendMessage = useCallback((text: string) => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN || !text.trim()) return;
    ws.send(JSON.stringify({ type: "chat_message", text: text.trim() }));
    setMessages((prev) => [
      ...prev,
      {
        id: `${Date.now()}-local`,
        peerId: localPeerIdRef.current ?? "me",
        text: text.trim(),
        timestampMs: Date.now(),
        isLocal: true,
      },
    ]);
  }, []);

  const disconnect = useCallback(() => {
    wsRef.current?.close();
    wsRef.current = null;
    pcRef.current?.close();
    pcRef.current = null;
    screenTrackRef.current?.stop();
    screenTrackRef.current = null;
    cameraTrackRef.current = null;
    localStreamRef.current?.getTracks().forEach((t) => t.stop());
    localStreamRef.current = null;
    remoteStreamsMapRef.current.clear();
    localPeerIdRef.current = null;
    setConnected(false);
    setIsScreenSharing(false);
    setIsMuted(false);
    setIsCameraOff(false);
    setLocalStream(null);
    setRemoteStreams([]);
    setMessages([]);
  }, []);

  const connect = useCallback(async () => {
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }
    pcRef.current?.close();
    pcRef.current = null;
    remoteStreamsMapRef.current.clear();
    setConnected(false);
    setError(null);
    setRemoteStreams([]);

    try {
      try {
        const { data } = await iceConfigApi.get();
        iceServersRef.current = data.ice_servers.map((s) => ({
          urls: s.urls,
          ...(s.username ? { username: s.username } : {}),
          ...(s.credential ? { credential: s.credential } : {}),
        }));
      } catch {
        iceServersRef.current = FALLBACK_ICE_SERVERS;
      }

      const stream = await navigator.mediaDevices.getUserMedia({
        video: true,
        audio: true,
      });
      localStreamRef.current = stream;
      cameraTrackRef.current = stream.getVideoTracks()[0] ?? null;
      setLocalStream(stream);

      const token = storage.getToken();
      if (!token) {
        setError("Not authenticated — please log in again");
        return;
      }

      // Single PC to the SFU server
      const pc = new RTCPeerConnection({ iceServers: iceServersRef.current });
      pcRef.current = pc;

      // Add local tracks so the server receives our audio+video
      stream.getTracks().forEach((t) => pc.addTrack(t, stream));

      // Remote tracks arrive here — stream.id equals the sender's peer_id (set by server via MSID)
      pc.ontrack = ({ streams, track }) => {
        const remoteStream = streams[0];
        if (!remoteStream) return;
        const peerId = remoteStream.id;
        if (!peerId) return;

        const existing = remoteStreamsMapRef.current.get(peerId);
        if (existing) {
          if (!existing.getTracks().find((t) => t.id === track.id)) {
            existing.addTrack(track);
          }
          // New MediaStream object so React detects the reference change and
          // VideoTile re-runs its effect to re-attach srcObject + call play().
          remoteStreamsMapRef.current.set(peerId, new MediaStream(existing.getTracks()));
        } else {
          remoteStreamsMapRef.current.set(peerId, remoteStream);
        }
        syncRemoteStreams();
      };

      pc.oniceconnectionstatechange = () => {
        console.log("[SFU] ICE state:", pc.iceConnectionState);
      };

      // Trickle ICE candidates to the server
      pc.onicecandidate = ({ candidate }) => {
        if (candidate) {
          console.log("[SFU] sending ICE candidate:", candidate.candidate);
          if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(
              JSON.stringify({ type: "ice_candidate", candidate: candidate.candidate })
            );
          }
        }
      };

      pc.onconnectionstatechange = () => {
        console.log("[SFU] PC connection state:", pc.connectionState);
        if (pc.connectionState === "failed") {
          setError("WebRTC connection failed — ICE may have timed out. Try rejoining.");
        }
        // Do NOT set connected=false here: connected reflects the WS session,
        // not the WebRTC media path. The WS onclose handler handles disconnection.
      };

      const ws = new WebSocket(`${WS_URL}/rooms/${roomId}/join?token=${token}`);
      wsRef.current = ws;

      ws.onmessage = async ({ data }) => {
        const msg = JSON.parse(data as string) as Record<string, unknown>;
        console.log("[SFU] WS message:", msg.type, msg);

        switch (msg.type) {
          // Server sends its SDP offer; we answer
          case "joined": {
            localPeerIdRef.current = (msg.peer_id as string) ?? null;
            setConnected(true);
            if (msg.sdp) await handleOffer(msg.sdp as string);
            break;
          }

          // Renegotiation offer when a new peer joins
          case "offer": {
            if (msg.sdp) await handleOffer(msg.sdp as string);
            break;
          }

          case "peer_joined":
            // UI notification — tracks arrive via ontrack
            break;

          case "peer_left": {
            const peerId = msg.peer_id as string | undefined;
            if (peerId) {
              remoteStreamsMapRef.current.delete(peerId);
              syncRemoteStreams();
            }
            break;
          }

          case "ice_candidate": {
            const candidate = msg.candidate as string | undefined;
            if (!candidate) break;
            const init: RTCIceCandidateInit = { candidate };
            const pc = pcRef.current;
            if (pc && pc.remoteDescription) {
              pc.addIceCandidate(init).catch(() => {});
            } else {
              pendingRemoteCandidates.current.push(init);
            }
            break;
          }

          case "chat_from": {
            const fromPeer = msg.from_peer as string | undefined;
            const text = msg.text as string | undefined;
            if (fromPeer && text) {
              setMessages((prev) => [
                ...prev,
                {
                  id: `${msg.timestamp_ms ?? Date.now()}-${fromPeer}`,
                  peerId: fromPeer,
                  text,
                  timestampMs: (msg.timestamp_ms as number) ?? Date.now(),
                  isLocal: false,
                },
              ]);
            }
            break;
          }

          case "error":
            setError((msg.reason as string) ?? "Unknown error");
            break;
        }
      };

      ws.onerror = () => setError("WebSocket error — check console and server logs");
      ws.onclose = () => setConnected(false);
    } catch (err) {
      setError(String(err));
    }
  }, [roomId, handleOffer, syncRemoteStreams]);

  useEffect(() => () => { disconnect(); }, [disconnect]);

  return {
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
  };
}
