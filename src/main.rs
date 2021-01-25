//use err_tools::*;
use hyper::service::{make_service_fn, service_fn, Service};
use hyper::{Body, Request, Response, Server};
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

pub struct Handle {
    d: Doer,
}

impl Service<Request<Body>> for Handle {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(async { handle(req).await })
    }
}

async fn handle(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    println!("Signal recieved");

    let (parts, _) = req.into_parts();

    let completer = Completer::from_uri(&parts.uri)?;

    let q = parts.uri.query().unwrap_or("No Query");

    let r = format!("Hello from '{}' you said: '{:?}' ", q, completer,);

    Ok(Response::new(Body::from(r)))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 9056));
    let doer = crate::manager::make_manager(&history::history_path());

    let make_service = make_service_fn(|_conn| async { Handle { d: doer.clone() } });

    let server = Server::bind(&addr).serve(make_service);

    // And run forever...
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
