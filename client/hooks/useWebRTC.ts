"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { storage } from "@/lib/storage";

const WS_URL = process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:4000";
const ICE_CONFIG: RTCConfiguration = {
  iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
};

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

  const [localStream, setLocalStream] = useState<MediaStream | null>(null);
  const [remoteStreams, setRemoteStreams] = useState<RemoteStream[]>([]);
  const [connected, setConnected] = useState(false);
  const [isScreenSharing, setIsScreenSharing] = useState(false);
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
      pcsRef.current.get(peerId)?.close();
      pcsRef.current.delete(peerId);
      remoteStreamsMapRef.current.delete(peerId);
      syncRemoteStreams();
    },
    [syncRemoteStreams]
  );

  const makePC = useCallback(
    (peerId: string): RTCPeerConnection => {
      const existing = pcsRef.current.get(peerId);
      if (existing) return existing;

      const pc = new RTCPeerConnection(ICE_CONFIG);
      pcsRef.current.set(peerId, pc);

      localStreamRef.current
        ?.getTracks()
        .forEach((t) => pc.addTrack(t, localStreamRef.current!));

      pc.ontrack = ({ streams, track }) => {
        const stream =
          streams[0] ??
          (() => {
            const s =
              remoteStreamsMapRef.current.get(peerId) ?? new MediaStream();
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
        if (
          pc.connectionState === "failed" ||
          pc.connectionState === "closed"
        ) {
          removePeer(peerId);
        }
      };

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
      const screenStream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
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

  const disconnect = useCallback(() => {
    wsRef.current?.close();
    wsRef.current = null;
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
    setLocalStream(null);
    setRemoteStreams([]);
  }, []);

  const connect = useCallback(async () => {
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }
    pcsRef.current.forEach((pc) => pc.close());
    pcsRef.current.clear();
    remoteStreamsMapRef.current.clear();
    setConnected(false);
    setError(null);
    setRemoteStreams([]);

    try {
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

          // This peer was already in the room before us — they'll initiate to us
          case "existing_peer":
            break;

          // A new peer just joined — we initiate the offer to them
          case "peer_joined": {
            if (!msg.peer_id) break;
            const pc = makePC(msg.peer_id);
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);
            ws.send(
              JSON.stringify({
                type: "offer_to",
                target: msg.peer_id,
                sdp: offer.sdp,
              })
            );
            break;
          }

          // Incoming offer — create and send an answer
          case "offer_from": {
            if (!msg.from_peer || !msg.sdp) break;
            const pc = makePC(msg.from_peer);
            await pc.setRemoteDescription({ type: "offer", sdp: msg.sdp });
            const answer = await pc.createAnswer();
            await pc.setLocalDescription(answer);
            ws.send(
              JSON.stringify({
                type: "answer_to",
                target: msg.from_peer,
                sdp: answer.sdp,
              })
            );
            break;
          }

          // Answer to our offer
          case "answer_from": {
            if (!msg.from_peer || !msg.sdp) break;
            const pc = pcsRef.current.get(msg.from_peer);
            if (pc) {
              await pc.setRemoteDescription({ type: "answer", sdp: msg.sdp });
            }
            break;
          }

          // ICE candidate from a peer
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

  return { localStream, remoteStreams, connected, isScreenSharing, error, connect, disconnect, startScreenShare, stopScreenShare };
}
