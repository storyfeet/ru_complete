use crate::history;
use crate::tab_complete::Complete;
use crate::Completer;
use async_std::stream::StreamExt;
use std::path::PathBuf;
use tokio::sync::mpsc::*;
use tokio::sync::oneshot;

pub type RMessage = (Completer, oneshot::Sender<Complete>);

#[derive(Clone)]
pub struct Doer {
    ch_s: Sender<RMessage>,
}

pub fn complete_manager(path: &str) -> Doer {
    let (ch_s, ch_r) = channel(10);
    tokio::spawn(running_manager(PathBuf::from(path), ch_r));
    Doer { ch_s }
}

async fn running_manager(p: PathBuf, mut r: Receiver<RMessage>) {
    let mut hist = history::Store::new();
    hist.load_history(&p).await;
    while let Some((cp, reply)) = r.recv().await {
        match cp.mode {
            "" => {}
            "path" => {}
            _ => {}
        }
    }
}
