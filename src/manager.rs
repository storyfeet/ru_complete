use crate::tab_complete::Complete;
use crate::Completer;
use tokio::sync::mpsc::*;

pub struct RMessage {
    cp: Completer,
    rt: Sender<Complete>,
}

#[derive(Clone)]
pub struct Doer {
    ch_s: Sender<RMessage>,
}

pub fn complete_manager(path: &str) -> Doer {}
