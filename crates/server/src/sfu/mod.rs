pub mod udp;

use std::{net::SocketAddr, sync::Arc};
use tokio::{net::UdpSocket, sync::mpsc};
use udp::{run_demux, DemuxControl};

#[derive(Clone)]
pub struct SfuState {
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub demux_ctrl: mpsc::Sender<DemuxControl>,
}

impl SfuState {
    pub async fn bind(addr: &str) -> anyhow::Result<Self> {
        let socket = Arc::new(UdpSocket::bind(addr).await?);
        let local_addr = socket.local_addr()?;
        let (ctrl_tx, ctrl_rx) = mpsc::channel(256);
        tokio::spawn(run_demux(socket.clone(), ctrl_rx));
        Ok(Self { socket, local_addr, demux_ctrl: ctrl_tx })
    }
}
