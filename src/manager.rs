use crate::history;
use crate::Completer;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::*;
use tokio::sync::oneshot;

pub type RMessage = (Completer, oneshot::Sender<Vec<String>>);

#[derive(Clone)]
pub struct Doer {
    ch_s: Sender<RMessage>,
}

impl Doer {
    pub async fn complete(&self, c: Completer) -> Option<Vec<String>> {
        let (o_s, o_r) = oneshot::channel();
        self.ch_s.send((c, o_s)).await.ok();
        o_r.await.ok()
    }

    pub async fn kill(&self) {
        let (o_s, o_r) = oneshot::channel();
        self.ch_s
            .send((
                Completer {
                    mode: "kill".to_string(),
                    s: String::new(),
                    pwd: String::new(),
                },
                o_s,
            ))
            .await
            .ok();
        o_r.await.ok();
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
    while let Some((cp, reply)) = r.recv().await {
        println!("message recieved : {:?}", cp);
        match cp.mode.as_ref() {
            "history" | "" => {
                let g = hist.guess(&cp.s, &cp.pwd);
                println!("g = {:?}", g);
                reply.send(g).ok();
            }
            "save" => {
                hist.push_command(cp.s, cp.pwd).ok();
                //save
                reply.send(Vec::new()).ok();
            }
            "kill" => {
                r.close();
            }
            _ => {
                reply.send(Vec::new()).ok();
            }
        }
    }
    //TODO Save tree
    println!("Manager Closing");
}
