//use err_tools::*;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn, Service};
use hyper::{Body, Request, Response, Server};
use tokio::io::{AsyncRead, AsyncReadExt};

//use std::convert::Infallible;
use core::task::{Context, Poll};
use manager::Doer;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
mod history;
mod manager;
mod pather;

#[derive(Debug, Clone)]
pub struct Completer {
    mode: &'static str,
    s: String,
    pwd: String,
}

impl Completer {
    pub fn from_uri(uri: &hyper::Uri) -> anyhow::Result<Self> {
        let mut res = Completer {
            mode: "",
            s: String::new(),
            pwd: String::new(),
        };
        let up = url::Url::parse(&format!("https://a?{}", uri.query().unwrap_or("")))?;
        for (k, v) in up.query_pairs() {
            match k.as_ref() {
                "mode" => match v.as_ref() {
                    "path" => res.mode = "path",
                    _ => {}
                },
                "s" => res.s = v.to_string(),
                "pwd" => res.pwd = v.to_string(),
                _ => {}
            }
            println!("k={},v={}", k, v);
        }
        Ok(res)
    }
}

/*
pub struct Handle {
    d: Doer,
}

impl<'a> Service<&'a AddrStream> for Handle {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, anyhow::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), anyhow::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: &'a AddrStream) -> Self::Future {
        Box::pin(async move {
            let mut inn = req.into_inner();
            let s = read_stream(&mut inn).await?;
            println!("Hello {}", s);
            Ok(Response::new(Body::from(s)))
        })
    }
}
*/
async fn handle(req: Request<Body>, dr: Doer) -> anyhow::Result<Response<Body>> {
    println!("Signal recieved");

    let (parts, _) = req.into_parts();

    let completer = Completer::from_uri(&parts.uri)?;

    let q = parts.uri.query().unwrap_or("No Query");

    let cp = dr.complete(completer).await;

    let r = format!("Hello from '{}' you said: '{:?}' ", q, cp);

    Ok(Response::new(Body::from(r)))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 9056));
    let doer = crate::manager::make_manager(&history::history_path());

    let make_service = make_service_fn(move |_conn| {
        let dr = doer.clone();
        async move {
            let d2 = dr.clone();
            Ok::<_, anyhow::Error>(service_fn(move |req| handle(req, d2.clone())))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

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
