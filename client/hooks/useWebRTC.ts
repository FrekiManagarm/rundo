"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { iceConfigApi } from "@/lib/api";
import { storage } from "@/lib/storage";

const WS_URL = process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:4000";

const FALLBACK_ICE_SERVERS: RTCIceServer[] = [
  { urls: "stun:stun.l.google.com:19302" },
];

interface ServerMessage {
  type: string;
  peer_id?: string;
  from_peer?: string;
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
  const localStreamRef = useRef<MediaStream | null>(null);
  const cameraTrackRef = useRef<MediaStreamTrack | null>(null);
  const screenTrackRef = useRef<MediaStreamTrack | null>(null);
  const pcsRef = useRef<Map<string, RTCPeerConnection>>(new Map());
  const remoteStreamsMapRef = useRef<Map<string, MediaStream>>(new Map());
  const iceServersRef = useRef<RTCIceServer[]>(FALLBACK_ICE_SERVERS);
  // Timers for delayed ICE restart on "disconnected" state
  const iceRestartTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const [localStream, setLocalStream] = useState<MediaStream | null>(null);
  const [remoteStreams, setRemoteStreams] = useState<RemoteStream[]>([]);
  const [connected, setConnected] = useState(false);
  const [isScreenSharing, setIsScreenSharing] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isCameraOff, setIsCameraOff] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const syncRemoteStreams = useCallback(() => {
    setRemoteStreams(
      Array.from(remoteStreamsMapRef.current.entries()).map(([peerId, stream]) => ({
        peerId,
        stream,
      }))
    );
  }, []);

  const removePeer = useCallback(
    (peerId: string) => {
      clearTimeout(iceRestartTimersRef.current.get(peerId));
      iceRestartTimersRef.current.delete(peerId);
      pcsRef.current.get(peerId)?.close();
      pcsRef.current.delete(peerId);
      remoteStreamsMapRef.current.delete(peerId);
      syncRemoteStreams();
    },
    [syncRemoteStreams]
  );

  // isOfferer: the peer who sent the offer is responsible for ICE restarts
  const makePC = useCallback(
    (peerId: string, isOfferer = false): RTCPeerConnection => {
      const existing = pcsRef.current.get(peerId);
      if (existing) return existing;

      const pc = new RTCPeerConnection({ iceServers: iceServersRef.current });
      pcsRef.current.set(peerId, pc);

      localStreamRef.current
        ?.getTracks()
        .forEach((t) => pc.addTrack(t, localStreamRef.current!));

      pc.ontrack = ({ streams, track }) => {
        const stream =
          streams[0] ??
          (() => {
            const s = remoteStreamsMapRef.current.get(peerId) ?? new MediaStream();
            s.addTrack(track);
            return s;
          })();
        remoteStreamsMapRef.current.set(peerId, stream);
        syncRemoteStreams();
      };

      pc.onicecandidate = ({ candidate }) => {
        if (candidate && wsRef.current?.readyState === WebSocket.OPEN) {
          wsRef.current.send(
            JSON.stringify({
              type: "ice_candidate_to",
              target: peerId,
              candidate: JSON.stringify(candidate),
            })
          );
        }
      };

      pc.onconnectionstatechange = () => {
        if (pc.connectionState === "failed" || pc.connectionState === "closed") {
          removePeer(peerId);
        }
      };

      // ICE restart — only the offerer re-initiates to avoid glare
      if (isOfferer) {
        const restartIce = async () => {
          if (wsRef.current?.readyState !== WebSocket.OPEN) return;
          try {
            const offer = await pc.createOffer({ iceRestart: true });
            await pc.setLocalDescription(offer);
            wsRef.current.send(
              JSON.stringify({ type: "offer_to", target: peerId, sdp: offer.sdp })
            );
          } catch {
            // PC may have been closed
          }
        };

        pc.oniceconnectionstatechange = () => {
          if (pc.iceConnectionState === "failed") {
            clearTimeout(iceRestartTimersRef.current.get(peerId));
            iceRestartTimersRef.current.delete(peerId);
            void restartIce();
          } else if (pc.iceConnectionState === "disconnected") {
            // Wait 5 s in case it self-recovers before forcing a restart
            const t = setTimeout(() => {
              if (
                pc.iceConnectionState === "disconnected" ||
                pc.iceConnectionState === "failed"
              ) {
                void restartIce();
              }
            }, 5000);
            iceRestartTimersRef.current.set(peerId, t);
          } else if (
            pc.iceConnectionState === "connected" ||
            pc.iceConnectionState === "completed"
          ) {
            clearTimeout(iceRestartTimersRef.current.get(peerId));
            iceRestartTimersRef.current.delete(peerId);
          }
        };
      }

      return pc;
    },
    [syncRemoteStreams, removePeer]
  );

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
        pcsRef.current.forEach((pc) => {
          const sender = pc.getSenders().find((s) => s.track?.kind === "video");
          if (sender) sender.replaceTrack(cameraTrack);
        });
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

      pcsRef.current.forEach((pc) => {
        const sender = pc.getSenders().find((s) => s.track?.kind === "video");
        if (sender) sender.replaceTrack(screenTrack);
      });

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
    const enabled = !isMuted;
    stream.getAudioTracks().forEach((t) => { t.enabled = enabled; });
    setIsMuted(!isMuted);
  }, [isMuted]);

  const toggleCamera = useCallback(() => {
    const stream = localStreamRef.current;
    if (!stream) return;
    const enabled = !isCameraOff;
    stream.getVideoTracks().forEach((t) => { t.enabled = enabled; });
    setIsCameraOff(!isCameraOff);
  }, [isCameraOff]);

  const disconnect = useCallback(() => {
    wsRef.current?.close();
    wsRef.current = null;
    iceRestartTimersRef.current.forEach((t) => clearTimeout(t));
    iceRestartTimersRef.current.clear();
    pcsRef.current.forEach((pc) => pc.close());
    pcsRef.current.clear();
    screenTrackRef.current?.stop();
    screenTrackRef.current = null;
    cameraTrackRef.current = null;
    localStreamRef.current?.getTracks().forEach((t) => t.stop());
    localStreamRef.current = null;
    remoteStreamsMapRef.current.clear();
    setConnected(false);
    setIsScreenSharing(false);
    setIsMuted(false);
    setIsCameraOff(false);
    setLocalStream(null);
    setRemoteStreams([]);
  }, []);

  const connect = useCallback(async () => {
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }
    iceRestartTimersRef.current.forEach((t) => clearTimeout(t));
    iceRestartTimersRef.current.clear();
    pcsRef.current.forEach((pc) => pc.close());
    pcsRef.current.clear();
    remoteStreamsMapRef.current.clear();
    setConnected(false);
    setError(null);
    setRemoteStreams([]);

    try {
      // Fetch ICE config (STUN + ephemeral TURN credentials) from server
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

      const ws = new WebSocket(`${WS_URL}/rooms/${roomId}/join?token=${token}`);
      wsRef.current = ws;

      ws.onmessage = async ({ data }) => {
        const msg: ServerMessage = JSON.parse(data);

        switch (msg.type) {
          case "joined":
            setConnected(true);
            break;

          case "existing_peer":
            break;

          // New peer joined — WE initiate (isOfferer = true → we handle ICE restarts)
          case "peer_joined": {
            if (!msg.peer_id) break;
            const pc = makePC(msg.peer_id, true);
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);
            ws.send(
              JSON.stringify({ type: "offer_to", target: msg.peer_id, sdp: offer.sdp })
            );
            break;
          }

          case "offer_from": {
            if (!msg.from_peer || !msg.sdp) break;
            const pc = makePC(msg.from_peer, false);
            await pc.setRemoteDescription({ type: "offer", sdp: msg.sdp });
            const answer = await pc.createAnswer();
            await pc.setLocalDescription(answer);
            ws.send(
              JSON.stringify({ type: "answer_to", target: msg.from_peer, sdp: answer.sdp })
            );
            break;
          }

          case "answer_from": {
            if (!msg.from_peer || !msg.sdp) break;
            const pc = pcsRef.current.get(msg.from_peer);
            if (pc) await pc.setRemoteDescription({ type: "answer", sdp: msg.sdp });
            break;
          }

          case "ice_candidate_from": {
            if (!msg.from_peer || !msg.candidate) break;
            const pc = pcsRef.current.get(msg.from_peer);
            if (pc) {
              try {
                await pc.addIceCandidate(JSON.parse(msg.candidate));
              } catch {
                // ignore stale candidates
              }
            }
            break;
          }

          case "peer_left":
            if (msg.peer_id) removePeer(msg.peer_id);
            break;

          case "error":
            setError(msg.reason ?? "Unknown error");
            break;
        }
      };

      ws.onerror = () =>
        setError("WebSocket error — check console and server logs");
      ws.onclose = () => setConnected(false);
    } catch (err) {
      setError(String(err));
    }
  }, [roomId, makePC, removePeer]);

  useEffect(() => () => { disconnect(); }, [disconnect]);

  return {
    localStream,
    remoteStreams,
    connected,
    isScreenSharing,
    isMuted,
    isCameraOff,
    error,
    connect,
    disconnect,
    toggleMic,
    toggleCamera,
    startScreenShare,
    stopScreenShare,
  };
}
