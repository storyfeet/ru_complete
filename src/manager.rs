use crate::history;
use crate::Completer;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
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

pub fn make_manager(path: &Path, killer: oneshot::Sender<()>) -> Doer {
    let (ch_s, ch_r) = channel::<RMessage>(10);
    tokio::spawn(running_manager(PathBuf::from(path), ch_r, killer));
    Doer { ch_s }
}

async fn running_manager(p: PathBuf, mut r: Receiver<RMessage>, killer: oneshot::Sender<()>) {
    let mut hist = history::Store::new();
    hist.load_history(&p).await;
    let mut last_save = SystemTime::now();
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
                println!("HIST Len = {}", hist.len());
                reply.send(Vec::new()).ok();
            }
            "kill" => {
                r.close();
            }
            _ => {
                reply.send(Vec::new()).ok();
            }
        }
        let t2 = SystemTime::now();
        //Save every half hour
        if t2
            .duration_since(last_save)
            .unwrap_or(Duration::from_secs(0))
            > Duration::from_secs(1800)
        {
            hist.save_append(history::date_to_history_path(std::time::SystemTime::now()))
                .await
                .ok();
            last_save = t2;
        }
    }
    hist.save_append(history::date_to_history_path(std::time::SystemTime::now()))
        .await
        .ok();
    //TODO Save tree
    killer.send(()).ok();
    println!("Manager Closing");
}
