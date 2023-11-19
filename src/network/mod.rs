pub mod smol_hyper;

use http_body_util::Full;

use std::net::TcpListener;

use crate::structs::{OutputValues, Clients, Arm};

use self::smol_hyper::SmolIo;
use futures_util::sink::SinkExt;
use smol::{prelude::*, Async};

use async_tungstenite::{
    tungstenite::{handshake::derive_accept_key, protocol::Role, Message}, 
    WebSocketStream
};

use eyre::Result;
use hyper::{
    service::service_fn, 
    Request, Response, StatusCode, 
    body::Bytes,
    header::{
        HeaderValue, 
        CONNECTION, UPGRADE, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY}, 
    server::conn::http1
};


pub async fn handle_clients(values: Arm<OutputValues>, clients: Clients) {
    let _span = tracy_client::span!("handle clients");
    let mut clients = clients.lock().unwrap();

    let values_lock = values.lock().unwrap();
    let serialized_values = serde_json::to_string(
        &(*values_lock)
    ).unwrap(); // Serialization not gonna fail in any possible way

    drop(values_lock); // Droping lock manually just in case

    clients.retain_mut(|websocket| {
        smol::block_on(async {
            let next_future = websocket.next();

            let msg_future = 
                smol::future::poll_once(next_future);

            let msg = match msg_future.await {
                Some(v) => {
                    match v {
                        Some(Ok(v)) => Some(v),
                        Some(Err(_)) => return false,
                        None => None,
                    }
                },
                None => None,
            };


            if let Some(Message::Close(_)) = msg {
                return false;
            };

            websocket.send(
                Message::Text(serialized_values.clone())
            ).await.unwrap();

            true
        })
    });
}

pub fn server_thread(ctx: Clients) {
    smol::block_on(async {
        let tcp = TcpListener::bind("127.0.0.1:9001").unwrap();
        let listener = Async::new(tcp)
            .unwrap();


        loop {
            let (stream, _) = listener.accept().await.unwrap();

            let io = SmolIo::new(stream);
            
            let ctx = ctx.clone();
            let service = service_fn(move |req| {
                let ctx = ctx.clone();
                serve(ctx, req)
            });

            smol::spawn(async {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            }).detach();
        }
    })
}

async fn serve_ws(
    clients: Clients, 
    mut req: Request<hyper::body::Incoming>
) -> Result<Response<Full<Bytes>>> {

    let headers = req.headers();
    let key = headers.get(SEC_WEBSOCKET_KEY);
    let derived = key.map(|k| derive_accept_key(k.as_bytes()));
    let ver = req.version();
    
    // It needs to be detached in order to upgrade properly work
    smol::spawn(async move {
        let upgraded = hyper::upgrade::on(&mut req).await
            .expect("Upgrade failed!");

        let upgraded = SmolIo::new(upgraded);

        let client = WebSocketStream::from_raw_socket(
            upgraded,
            Role::Server,
            None,
        ).await;
        
        let mut clients = clients.lock().unwrap();

        clients.push(client);
    }).detach();
    
    let mut res = Response::new(Full::new(Bytes::default()));

    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *res.version_mut() = ver;

    res.headers_mut().append(
        CONNECTION, HeaderValue::from_static("Upgrade")
    );

    res.headers_mut().append(
        UPGRADE, HeaderValue::from_static("websocket")
    );

    res.headers_mut()
        .append(
            SEC_WEBSOCKET_ACCEPT, 
            derived.unwrap().parse().unwrap() //TODO remove unwraps
    );

    Ok(res)
}

async fn serve_http(
    _clients: Clients, 
    mut _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>> {
    Ok(Response::new(Full::new(Bytes::from("Hello World!"))))
}

async fn serve(
    clients: Clients, 
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>> {
    if req.uri() != "/ws" {
        serve_http(clients, req).await
    } else {
        serve_ws(clients, req).await
    }
}
