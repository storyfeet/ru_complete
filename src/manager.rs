use crate::tab_complete as tc;
use crate::tab_complete::Complete;
use crate::Completer;
use std::path::PathBuf;
use tokio::sync::mpsc::*;

pub struct RMessage {
    cp: Completer,
    rt: Sender<Complete>,
}

#[derive(Clone)]
pub struct Doer {
    ch_s: Sender<RMessage>,
}

pub fn complete_manager(path: &str) -> Doer {
    let (ch_s, ch_r) = channel(10);
    tokio::spawn(running_manager(PathBuf::from(path), ch_r));
    Doer { ch_s }
}

async fn running_manager(p: PathBuf, r: Receiver<RMessage>) {
    let mut hist = tc::HistoryStore::new();
    hist.load_history(&p)
}
