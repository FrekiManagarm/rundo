use std::{net::SocketAddr, sync::Arc, time::Instant};

use anyhow::Result;
use shared::models::PeerId;
use str0m::{
    change::SdpOffer,
    net::{Protocol, Receive},
    Event, Input, Output, Rtc, RtcConfig,
};
use tokio::{net::UdpSocket, sync::mpsc};

use crate::{
    rooms::registry::{RoomCommand, RtpPayload},
    sfu::udp::DemuxControl,
};

pub struct Peer {
    pub id: PeerId,
    rtc: Rtc,
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}

impl Peer {
    pub fn new(
        id: PeerId,
        socket: Arc<UdpSocket>,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Self {
        let rtc = RtcConfig::new().set_ice_lite(true).build();
        Self { id, rtc, socket, local_addr, remote_addr }
    }

    pub fn accept_offer(&mut self, sdp: &str) -> Result<String> {
        let offer = SdpOffer::from_sdp_string(sdp)?;
        let answer = self.rtc.sdp_api().accept_offer(offer)?;
        Ok(answer.to_sdp_string())
    }
}

/// Arguments for spawning a peer task. Grouped to stay under the clippy argument limit.
pub struct PeerTask {
    pub peer_id: PeerId,
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub sdp_offer: String,
    pub room_cmd_tx: mpsc::Sender<RoomCommand>,
    pub demux_ctrl: mpsc::Sender<DemuxControl>,
    pub udp_rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
    pub rtp_rx: mpsc::Receiver<RtpPayload>,
    pub answer_tx: tokio::sync::oneshot::Sender<Result<String>>,
}

/// Runs a Peer's event loop.
///
/// The signaling session must register `udp_tx` with the demux BEFORE spawning this task.
pub async fn run_peer(args: PeerTask) {
    let PeerTask {
        peer_id,
        socket,
        local_addr,
        remote_addr,
        sdp_offer,
        room_cmd_tx,
        demux_ctrl,
        mut udp_rx,
        mut rtp_rx,
        answer_tx,
    } = args;

    let mut peer = Peer::new(peer_id, socket, local_addr, remote_addr);

    let answer = match peer.accept_offer(&sdp_offer) {
        Ok(a) => a,
        Err(e) => {
            let _ = answer_tx.send(Err(e));
            return;
        }
    };
    let _ = answer_tx.send(Ok(answer));

    let socket = peer.socket.clone();

    loop {
        let timeout = match peer.rtc.poll_output() {
            Ok(Output::Timeout(t)) => t,
            Ok(Output::Transmit(t)) => {
                let _ = socket.send_to(&t.contents, t.destination).await;
                continue;
            }
            Ok(Output::Event(Event::MediaData(data))) => {
                let payload = RtpPayload {
                    data: data.data,
                    timestamp: data.time.numer() as u32,
                    payload_type: *data.pt,
                };
                let _ = room_cmd_tx
                    .send(RoomCommand::MediaData { from: peer_id, payload })
                    .await;
                continue;
            }
            Ok(Output::Event(_)) => continue,
            Err(e) => {
                tracing::warn!("peer {peer_id:?} rtc error: {e}");
                break;
            }
        };

        let sleep_dur = timeout.saturating_duration_since(Instant::now());

        tokio::select! {
            _ = tokio::time::sleep(sleep_dur) => {
                let _ = peer.rtc.handle_input(Input::Timeout(Instant::now()));
            }
            Some((src, data)) = udp_rx.recv() => {
                if let Ok(receive) = Receive::new(Protocol::Udp, src, peer.local_addr, &data) {
                    let _ = peer.rtc.handle_input(Input::Receive(Instant::now(), receive));
                }
            }
            Some(rtp) = rtp_rx.recv() => {
                tracing::debug!("peer {peer_id:?} received forwarded RTP ({} bytes)", rtp.data.len());
            }
        }
    }

    let _ = demux_ctrl.send(DemuxControl::Unregister { addr: peer.remote_addr }).await;
    let _ = room_cmd_tx.send(RoomCommand::PeerLeft { peer_id }).await;
}
