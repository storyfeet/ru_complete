//use err_tools::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
//use tokio::io::{AsyncRead, AsyncReadExt};

//use std::convert::Infallible;
use manager::Doer;
use std::net::SocketAddr;
mod history;
mod manager;
mod pather;

#[derive(Debug, Clone)]
pub struct Completer {
    mode: String,
    s: String,
    pwd: String,
}

impl Completer {
    pub fn from_uri(uri: &hyper::Uri) -> anyhow::Result<Self> {
        let mut res = Completer {
            mode: String::new(),
            s: String::new(),
            pwd: String::new(),
        };
        let up = url::Url::parse(&format!("https://a?{}", uri.query().unwrap_or("")))?;
        for (k, v) in up.query_pairs() {
            match k.as_ref() {
                "mode" => res.mode = v.to_string(),
                "s" => res.s = v.to_string(),
                "pwd" => res.pwd = v.to_string(),
                _ => {}
            }
        }
        Ok(res)
    }
}

async fn handle(req: Request<Body>, dr: Doer) -> anyhow::Result<Response<Body>> {
    let (parts, _) = req.into_parts();

    let completer = Completer::from_uri(&parts.uri)?;

    let cp = dr.complete(completer).await.unwrap_or(Vec::new());

    let s = serde_json::to_string(&cp).unwrap_or("[]".to_string());

    Response::builder()
        .header("Content-Type", "text/json")
        .body(Body::from(s))
        .map_err(|e| e.into())
}

async fn signal_handler(k: SignalKind, doer: Doer, s_killer: mpsc::Sender<()>) {
    let mut stream = match signal(k) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not listen for signal: {}", e);
            return;
        }
    };
    loop {
        stream.recv().await;
        println!("Kill revieved");
        doer.kill().await;
        s_killer.send(()).await.ok();
    }
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 9056));
    let doer = crate::manager::make_manager(&history::history_path());
    let (kill_s, mut kill_r) = mpsc::channel(1);
    tokio::spawn(signal_handler(
        SignalKind::interrupt(),
        doer.clone(),
        kill_s.clone(),
    ));

    let make_service = make_service_fn(move |_conn| {
        let dr = doer.clone();
        async move {
            let d2 = dr.clone();
            Ok::<_, anyhow::Error>(service_fn(move |req| handle(req, d2.clone())))
        }
    });

    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(async {
            kill_r.recv().await;
        });

    // And run forever...
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

/*async fn read_stream<A: AsyncRead + Unpin>(a: &mut A) -> anyhow::Result<String> {
    let mut b = [0u8; 20];
    let mut v = Vec::new();
    loop {
        match a.read(&mut b).await {
            Ok(0) => {
                return String::from_utf8(v).map_err(|e| e.into());
            }
            Ok(n) => v.extend(&b[0..n]),
            Err(e) => return Err(e.into()),
        }
    }
}*/
