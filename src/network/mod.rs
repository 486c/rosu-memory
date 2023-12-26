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

    let serialized_values = {
        let values_lock = values.lock().unwrap();
    
        serde_json::to_string(&*values_lock).unwrap()
    };

    let mut clients = clients.lock().unwrap();
    clients.retain_mut(|websocket| {
        smol::block_on(async {
            let next_future = websocket.next();

            let msg_future = 
                smol::future::poll_once(next_future);

            let msg = match msg_future.await {
                Some(Some(Ok(v))) => Some(v),
                Some(Some(Err(_))) => return false,
                Some(None) | None => None,
            };

            if let Some(Message::Close(_)) = msg {
                return false;
            };

            let sent = websocket.send(
                Message::Text(serialized_values.clone())
            ).await;
            match sent {
                Ok(()) => {},
                Err(error) => { println!("{error}") }
            }

            true
        })
    });
}

pub fn server_thread(ctx_clients: Clients, values: Arm<OutputValues>) {
    smol::block_on(async {
        let tcp = TcpListener::bind("127.0.0.1:9001").unwrap();
        let listener = Async::new(tcp)
            .unwrap();


        loop {
            let (stream, _) = listener.accept().await.unwrap();

            let io = SmolIo::new(stream);
            
            let ctx_clients = ctx_clients.clone();
            let ctx_values = values.clone();
            let service = service_fn(move |req| {
                let ctx_clients = ctx_clients.clone();
                let ctx_values = ctx_values.clone();
                serve(ctx_clients, ctx_values, req)
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
    values: Arm<OutputValues>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>> {
    let mut path = req.uri().path().splitn(3, '/').skip(1);

    let songs_path = match path.next() {
        Some(v) => v,
        None => return Ok(
            Response::builder()
                .status(404)
                .body(Full::default())?
        ),
    };

    if songs_path.starts_with("Songs") {
        let background_path = { 
            let values = values.lock().unwrap();

            values.background_path_full.clone()
        };

        if !background_path.exists() {
            return Ok(Response::builder()
                .status(400)
                .body(Full::default())?
            )
        }

        let bytes = smol::fs::read(background_path).await?;
        
        Ok(Response::new(Full::new(
            Bytes::copy_from_slice(bytes.as_slice())
        )))
    } else {
        Ok(Response::builder()
            .status(400)
            .body(Full::default())?
        )
    }
}

async fn serve(
    clients: Clients, 
    values: Arm<OutputValues>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>> {
    if req.uri() != "/ws" {
        serve_http(values, req).await
    } else {
        serve_ws(clients, req).await
    }
}
