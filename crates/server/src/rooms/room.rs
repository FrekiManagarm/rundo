use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use shared::{messages::ServerMessage, models::{PeerId, RoomId}};
use tokio::sync::{mpsc, RwLock};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_OPUS, MIME_TYPE_VP8},
        APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration,
        sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::{
        rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType},
        rtp_transceiver_direction::RTCRtpTransceiverDirection,
        RTCRtpTransceiverInit,
    },
    track::track_local::{
        track_local_static_rtp::TrackLocalStaticRTP,
        TrackLocal, TrackLocalWriter,
    },
};

use crate::rooms::registry::RoomCommand;

// ── internal events ──────────────────────────────────────────────────────────

enum RoomEvent {
    TrackReady { peer_id: PeerId, kind: RTPCodecType },
}

// ── forward list: send-tracks that each incoming RTP packet is routed to ────

type FwdList = Arc<RwLock<Vec<Arc<TrackLocalStaticRTP>>>>;

// ── per-peer state ────────────────────────────────────────────────────────────

struct AudioVideoTracks {
    audio: Arc<TrackLocalStaticRTP>,
    video: Arc<TrackLocalStaticRTP>,
}

struct SfuPeer {
    peer_id:        PeerId,
    pc:             Arc<RTCPeerConnection>,
    ws_tx:          mpsc::Sender<ServerMessage>,
    /// SendOnly tracks inside this PC keyed by the source peer's id
    outgoing:       HashMap<PeerId, AudioVideoTracks>,
    audio_ready:    bool,
    video_ready:    bool,
    /// true once we've triggered renegotiation for this peer's media to others
    renegotiated:   bool,
    /// true while waiting for browser's SDP answer
    pending_answer: bool,
    /// Forward lists: packets from this peer's incoming tracks are written here
    fwd_audio: FwdList,
    fwd_video: FwdList,
}

type PeersMap = Arc<RwLock<HashMap<PeerId, SfuPeer>>>;

// ── room entry point ──────────────────────────────────────────────────────────

pub async fn run_room(
    room_id: RoomId,
    mut cmd_rx: mpsc::Receiver<RoomCommand>,
    peer_counter: Arc<AtomicUsize>,
    _server_addr: String,
) {
    let peers: PeersMap = Arc::new(RwLock::new(HashMap::new()));
    let (evt_tx, mut evt_rx) = mpsc::channel::<RoomEvent>(256);

    tracing::info!("room {room_id:?} started");

    loop {
        tokio::select! {
            biased;

            Some(cmd) = cmd_rx.recv() => {
                handle_command(cmd, &peers, &evt_tx, &peer_counter).await;
            }

            Some(evt) = evt_rx.recv() => {
                handle_room_event(evt, &peers).await;
            }

            else => break,
        }
    }

    tracing::info!("room {room_id:?} shutting down");
}

// ── command dispatch ──────────────────────────────────────────────────────────

async fn handle_command(
    cmd: RoomCommand,
    peers: &PeersMap,
    evt_tx: &mpsc::Sender<RoomEvent>,
    peer_counter: &Arc<AtomicUsize>,
) {
    match cmd {
        RoomCommand::PeerJoined { peer_id, info: _, ws_tx } => {
            add_peer(peer_id, ws_tx, peers, evt_tx.clone()).await;
            peer_counter.store(peers.read().await.len(), Ordering::Relaxed);
        }

        RoomCommand::PeerLeft { peer_id } => {
            let removed = peers.write().await.remove(&peer_id);
            if let Some(peer) = removed {
                let _ = peer.pc.close().await;
                let map = peers.read().await;
                for other in map.values() {
                    let _ = other.ws_tx.try_send(ServerMessage::PeerLeft { peer_id });
                }
            }
            peer_counter.store(peers.read().await.len(), Ordering::Relaxed);
        }

        RoomCommand::PeerAnswer { peer_id, sdp } => {
            let pc = {
                let mut map = peers.write().await;
                map.get_mut(&peer_id).map(|p| {
                    p.pending_answer = false;
                    p.pc.clone()
                })
            };
            if let Some(pc) = pc {
                match RTCSessionDescription::answer(sdp) {
                    Ok(desc) => {
                        if let Err(e) = pc.set_remote_description(desc).await {
                            tracing::warn!("peer {peer_id:?} set_remote_description: {e}");
                        } else {
                            // Catch up any tracks missed while pending_answer was true
                            catch_up_peer(peer_id, peers).await;
                        }
                    }
                    Err(e) => tracing::warn!("peer {peer_id:?} bad answer SDP: {e}"),
                }
            }
        }

        RoomCommand::PeerIceCandidate { peer_id, candidate } => {
            let pc = peers.read().await.get(&peer_id).map(|p| p.pc.clone());
            if let Some(pc) = pc {
                let init = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
                    candidate,
                    sdp_mid: None,
                    sdp_mline_index: None,
                    username_fragment: None,
                };
                if let Err(e) = pc.add_ice_candidate(init).await {
                    tracing::warn!("peer {peer_id:?} add_ice_candidate: {e}");
                }
            }
        }

        RoomCommand::BroadcastChat { from_peer, text, timestamp_ms } => {
            let map = peers.read().await;
            let msg = ServerMessage::ChatFrom { from_peer, text, timestamp_ms };
            for (&pid, peer) in map.iter() {
                if pid != from_peer {
                    let _ = peer.ws_tx.try_send(msg.clone());
                }
            }
        }
    }
}

// ── internal event handler ───────────────────────────────────────────────────

async fn handle_room_event(evt: RoomEvent, peers: &PeersMap) {
    let RoomEvent::TrackReady { peer_id, kind } = evt;

    let should_renegotiate = {
        let mut map = peers.write().await;
        if let Some(peer) = map.get_mut(&peer_id) {
            match kind {
                RTPCodecType::Audio => peer.audio_ready = true,
                RTPCodecType::Video => peer.video_ready = true,
                _ => {}
            }
            peer.audio_ready && peer.video_ready && !peer.renegotiated
        } else {
            false
        }
    };

    if should_renegotiate {
        renegotiate_all(peer_id, peers).await;
        if let Some(peer) = peers.write().await.get_mut(&peer_id) {
            peer.renegotiated = true;
        }
    }
}

// ── join a new peer ──────────────────────────────────────────────────────────

async fn add_peer(
    peer_id: PeerId,
    ws_tx: mpsc::Sender<ServerMessage>,
    peers: &PeersMap,
    evt_tx: mpsc::Sender<RoomEvent>,
) {
    let pc = match build_peer_connection().await {
        Ok(pc) => pc,
        Err(e) => {
            tracing::error!("peer {peer_id:?} build PC: {e}");
            let _ = ws_tx.try_send(ServerMessage::Error { reason: "WebRTC setup failed".into() });
            return;
        }
    };

    // RecvOnly transceivers — browser → server
    for kind in [RTPCodecType::Audio, RTPCodecType::Video] {
        if let Err(e) = pc.add_transceiver_from_kind(
            kind,
            Some(RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Recvonly,
                send_encodings: vec![],
            }),
        ).await {
            tracing::error!("peer {peer_id:?} add recvonly transceiver: {e}");
            return;
        }
    }

    // Forward lists for this peer's incoming media
    let fwd_audio: FwdList = Arc::new(RwLock::new(Vec::new()));
    let fwd_video: FwdList = Arc::new(RwLock::new(Vec::new()));

    // on_track: spawn a forwarding task for each incoming track
    {
        let fa = fwd_audio.clone();
        let fv = fwd_video.clone();
        let tx = evt_tx.clone();
        pc.on_track(Box::new(move |track, _, _| {
            let kind = track.kind();
            let fwd: FwdList = match kind {
                RTPCodecType::Audio => fa.clone(),
                _ => fv.clone(),
            };
            let tx = tx.clone();
            Box::pin(async move {
                tokio::spawn(async move {
                    loop {
                        match track.read_rtp().await {
                            Ok((pkt, _)) => {
                                let list = fwd.read().await;
                                for t in list.iter() {
                                    let _ = t.write_rtp(&pkt).await;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });
                let _ = tx.send(RoomEvent::TrackReady { peer_id, kind }).await;
            })
        }));
    }

    // Trickle ICE: forward each candidate to the browser as soon as it's gathered
    {
        let ws_tx_ice = ws_tx.clone();
        pc.on_ice_candidate(Box::new(move |candidate| {
            let ws_tx = ws_tx_ice.clone();
            Box::pin(async move {
                if let Some(c) = candidate {
                    if let Ok(init) = c.to_json() {
                        let _ = ws_tx.try_send(ServerMessage::IceCandidate {
                            candidate: init.candidate,
                        });
                    }
                }
            })
        }));
    }

    // Collect already-established peers to pre-populate sendonly tracks for them
    let established: Vec<(PeerId, FwdList, FwdList)> = {
        let map = peers.read().await;
        map.values()
            .filter(|p| p.renegotiated)
            .map(|p| (p.peer_id, p.fwd_audio.clone(), p.fwd_video.clone()))
            .collect()
    };

    // Add SendOnly tracks for each established peer into the new PC
    let mut outgoing: HashMap<PeerId, AudioVideoTracks> = HashMap::new();
    for (existing_id, fwd_a, fwd_v) in established {
        let audio_t = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability { mime_type: MIME_TYPE_OPUS.to_owned(), ..Default::default() },
            format!("audio-{}", existing_id.0),
            existing_id.0.to_string(),
        ));
        let video_t = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability { mime_type: MIME_TYPE_VP8.to_owned(), ..Default::default() },
            format!("video-{}", existing_id.0),
            existing_id.0.to_string(),
        ));
        let a_ok = pc.add_track(Arc::clone(&audio_t) as Arc<dyn TrackLocal + Send + Sync>).await.is_ok();
        let v_ok = pc.add_track(Arc::clone(&video_t) as Arc<dyn TrackLocal + Send + Sync>).await.is_ok();
        if a_ok && v_ok {
            fwd_a.write().await.push(audio_t.clone());
            fwd_v.write().await.push(video_t.clone());
            outgoing.insert(existing_id, AudioVideoTracks { audio: audio_t, video: video_t });
        }
    }

    // Notify existing peers of the new arrival
    {
        let map = peers.read().await;
        for peer in map.values() {
            let _ = peer.ws_tx.try_send(ServerMessage::PeerJoined { peer_id });
        }
    }

    // Create offer (ICE gathering happens here — no lock held)
    let offer_sdp = match create_offer(&pc).await {
        Ok(sdp) => sdp,
        Err(e) => {
            tracing::error!("peer {peer_id:?} create offer: {e}");
            let _ = ws_tx.try_send(ServerMessage::Error { reason: "Offer failed".into() });
            return;
        }
    };

    let _ = ws_tx.try_send(ServerMessage::Joined { peer_id, sdp: offer_sdp });

    peers.write().await.insert(peer_id, SfuPeer {
        peer_id,
        pc,
        ws_tx,
        outgoing,
        audio_ready: false,
        video_ready: false,
        renegotiated: false,
        pending_answer: true,
        fwd_audio,
        fwd_video,
    });
}

// ── renegotiate existing peers to add a newly-ready peer's tracks ─────────────

async fn renegotiate_all(new_peer_id: PeerId, peers: &PeersMap) {
    let (fwd_a, fwd_v, dsts) = {
        let map = peers.read().await;
        let Some(np) = map.get(&new_peer_id) else { return };
        let fwd_a = np.fwd_audio.clone();
        let fwd_v = np.fwd_video.clone();
        let dsts: Vec<(PeerId, Arc<RTCPeerConnection>, mpsc::Sender<ServerMessage>)> = map
            .values()
            .filter(|p| {
                p.peer_id != new_peer_id
                    && !p.pending_answer
                    && !p.outgoing.contains_key(&new_peer_id)
            })
            .map(|p| (p.peer_id, p.pc.clone(), p.ws_tx.clone()))
            .collect();
        (fwd_a, fwd_v, dsts)
    };

    for (dst_id, pc, ws_tx) in dsts {
        let audio_t = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability { mime_type: MIME_TYPE_OPUS.to_owned(), ..Default::default() },
            format!("audio-{}", new_peer_id.0),
            new_peer_id.0.to_string(),
        ));
        let video_t = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability { mime_type: MIME_TYPE_VP8.to_owned(), ..Default::default() },
            format!("video-{}", new_peer_id.0),
            new_peer_id.0.to_string(),
        ));

        let a_ok = pc.add_track(Arc::clone(&audio_t) as Arc<dyn TrackLocal + Send + Sync>).await.is_ok();
        let v_ok = pc.add_track(Arc::clone(&video_t) as Arc<dyn TrackLocal + Send + Sync>).await.is_ok();
        if !a_ok || !v_ok {
            continue;
        }

        fwd_a.write().await.push(audio_t.clone());
        fwd_v.write().await.push(video_t.clone());

        {
            let mut map = peers.write().await;
            if let Some(dst) = map.get_mut(&dst_id) {
                dst.outgoing.insert(new_peer_id, AudioVideoTracks { audio: audio_t, video: video_t });
                dst.pending_answer = true;
            }
        }

        match create_offer(&pc).await {
            Ok(sdp) => { let _ = ws_tx.try_send(ServerMessage::Offer { sdp }); }
            Err(e) => tracing::warn!("renegotiate {dst_id:?}: {e}"),
        }
    }
}

// ── catch-up: add tracks that were skipped while peer was pending_answer ─────

async fn catch_up_peer(peer_id: PeerId, peers: &PeersMap) {
    // Collect the peer's PC, ws_tx, and all renegotiated peers whose tracks
    // are not yet in this peer's outgoing map.
    let (peer_pc, peer_ws_tx, missing) = {
        let map = peers.read().await;
        let Some(peer) = map.get(&peer_id) else { return };
        let pc = peer.pc.clone();
        let ws_tx = peer.ws_tx.clone();
        let missing: Vec<(PeerId, FwdList, FwdList)> = map
            .values()
            .filter(|p| {
                p.peer_id != peer_id
                    && p.renegotiated
                    && !peer.outgoing.contains_key(&p.peer_id)
            })
            .map(|p| (p.peer_id, p.fwd_audio.clone(), p.fwd_video.clone()))
            .collect();
        (pc, ws_tx, missing)
    };

    if missing.is_empty() {
        return;
    }

    let mut added: Vec<(PeerId, Arc<TrackLocalStaticRTP>, Arc<TrackLocalStaticRTP>, FwdList, FwdList)> = vec![];

    for (src_id, fwd_a, fwd_v) in missing {
        let audio_t = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability { mime_type: MIME_TYPE_OPUS.to_owned(), ..Default::default() },
            format!("audio-{}", src_id.0),
            src_id.0.to_string(),
        ));
        let video_t = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability { mime_type: MIME_TYPE_VP8.to_owned(), ..Default::default() },
            format!("video-{}", src_id.0),
            src_id.0.to_string(),
        ));
        let a_ok = peer_pc.add_track(Arc::clone(&audio_t) as Arc<dyn TrackLocal + Send + Sync>).await.is_ok();
        let v_ok = peer_pc.add_track(Arc::clone(&video_t) as Arc<dyn TrackLocal + Send + Sync>).await.is_ok();
        if a_ok && v_ok {
            added.push((src_id, audio_t, video_t, fwd_a, fwd_v));
        }
    }

    if added.is_empty() {
        return;
    }

    for (src_id, audio_t, video_t, fwd_a, fwd_v) in &added {
        fwd_a.write().await.push(audio_t.clone());
        fwd_v.write().await.push(video_t.clone());
        let mut map = peers.write().await;
        if let Some(peer) = map.get_mut(&peer_id) {
            peer.outgoing.insert(*src_id, AudioVideoTracks { audio: audio_t.clone(), video: video_t.clone() });
            peer.pending_answer = true;
        }
    }

    match create_offer(&peer_pc).await {
        Ok(sdp) => { let _ = peer_ws_tx.try_send(ServerMessage::Offer { sdp }); }
        Err(e) => {
            tracing::warn!("catch_up {peer_id:?}: {e}");
            if let Some(peer) = peers.write().await.get_mut(&peer_id) {
                peer.pending_answer = false;
            }
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

async fn create_offer(pc: &RTCPeerConnection) -> anyhow::Result<String> {
    let offer = pc.create_offer(None).await?;
    pc.set_local_description(offer).await?;
    let local_desc = pc
        .local_description()
        .await
        .ok_or_else(|| anyhow::anyhow!("no local description"))?;
    Ok(local_desc.sdp)
}

async fn build_peer_connection() -> anyhow::Result<Arc<RTCPeerConnection>> {
    let mut media_engine = MediaEngine::default();

    // Register only Opus + VP8 with fixed payload types so every peer connection
    // uses the same PT values. This avoids payload-type mismatches when we forward
    // raw RTP packets between connections (VP9/H264 would use different PTs).
    media_engine.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 111,
            ..Default::default()
        },
        RTPCodecType::Audio,
    )?;
    media_engine.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: String::new(),
                rtcp_feedback: vec![],
            },
            payload_type: 96,
            ..Default::default()
        },
        RTPCodecType::Video,
    )?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)?;
    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let pc = api.new_peer_connection(config).await?;
    Ok(Arc::new(pc))
}
