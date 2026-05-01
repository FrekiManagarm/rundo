use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{net::UdpSocket, sync::mpsc};

pub enum DemuxControl {
    Register { addr: SocketAddr, tx: mpsc::Sender<(SocketAddr, Vec<u8>)> },
    Unregister { addr: SocketAddr },
}

pub async fn run_demux(socket: Arc<UdpSocket>, mut ctrl_rx: mpsc::Receiver<DemuxControl>) {
    let mut routes: HashMap<SocketAddr, mpsc::Sender<(SocketAddr, Vec<u8>)>> = HashMap::new();
    let mut buf = vec![0u8; 65535];

    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((len, src)) => {
                        let data = buf[..len].to_vec();
                        if let Some(tx) = routes.get(&src) {
                            let _ = tx.try_send((src, data));
                        }
                    }
                    Err(e) => tracing::error!("UDP recv error: {e}"),
                }
            }
            ctrl = ctrl_rx.recv() => {
                match ctrl {
                    Some(DemuxControl::Register { addr, tx }) => { routes.insert(addr, tx); }
                    Some(DemuxControl::Unregister { addr }) => { routes.remove(&addr); }
                    None => break,
                }
            }
        }
    }
}
