use crate::history;
use crate::pather::Complete;
use crate::Completer;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::*;
use tokio::sync::oneshot;

pub type RMessage = (Completer, oneshot::Sender<Complete>);

#[derive(Clone)]
pub struct Doer {
    ch_s: Sender<RMessage>,
}

impl Doer {
    pub async fn complete(&self, c: Completer) -> Option<Complete> {
        let (o_s, o_r) = oneshot::channel();
        self.ch_s.send((c, o_s)).await.ok();
        o_r.await.ok()
    }
}

pub fn make_manager(path: &Path) -> Doer {
    let (ch_s, ch_r) = channel::<RMessage>(10);
    tokio::spawn(running_manager(PathBuf::from(path), ch_r));
    Doer { ch_s }
}

async fn running_manager(p: PathBuf, mut r: Receiver<RMessage>) {
    let mut hist = history::Store::new();
    hist.load_history(&p).await;
    let mut n = 0;
    while let Some((cp, reply)) = r.recv().await {
        println!("message recieved : {:?}", cp);
        match cp.mode {
            "" => {}
            "path" => {}
            _ => {}
        }
        reply.send(Complete::One(format!("Hello {}", n))).ok();
        n += 1;
    }
}
