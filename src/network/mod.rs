mod smol_executor;

use std::{net::TcpListener, collections::HashMap, sync::{Arc, Mutex}};
use self::smol_executor::*;
use async_compat::*;
use futures_util::sink::SinkExt;
use smol::{prelude::*, Async};

use async_tungstenite::{tungstenite::{handshake::derive_accept_key, protocol::Role, Message}, WebSocketStream};
use eyre::{Result, Error};
use hyper::{Server, service::{make_service_fn, service_fn}, Request, Body, Response, StatusCode, header::{HeaderValue, CONNECTION, UPGRADE, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY}, upgrade::Upgraded, server::conn::Http};

struct Context {
    clients: Mutex<HashMap<usize, WebSocketStream<Compat<Upgraded>>>>,
}

pub fn server_thread() {
    smol::block_on(async {
        let listener = Async::new(TcpListener::bind("127.0.0.1:9001").unwrap())
            .unwrap();

        let server_context = Arc::new(Context {
            clients: Mutex::new(HashMap::new())
        });

        /*

        loop {
            let (stream, _) = listener.accept()
                .await.unwrap();

            let ctx = server_context.clone();

            let service = service_fn(move |req| {

                let ctx = ctx.clone();

                async {
                    Ok::<_, Error>(serve(ctx, req))
                }
            });

            Http::new()
                .serve_connection(stream.compat(), service).await
        }
        */

        let srv_ctx = server_context.clone();
        let server = Server::builder(SmolListener::new(&listener))
                .executor(SmolExecutor)
                .serve(make_service_fn(|_| {
                    let ctx = srv_ctx.clone();
                    async { Ok::<_, Error>(service_fn( move |req| {
                        let ctx = ctx.clone();
                        serve(ctx, req)
                    })) }
                }));
        
        let wbs_ctx = server_context.clone();
        let websocket_loop = async move {
            let ctx = wbs_ctx.clone();
            
            loop {
                let mut clients = ctx.clients.lock().unwrap();
                clients.retain(|_client_id, websocket| {
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
                            Message::Text(
                                "hii".to_string()
                                )
                            ).await.unwrap();

                        true
                    })
                });
            }
        };

        smol::spawn(websocket_loop).detach();

        server.await.unwrap();
    })
}

async fn serve(ctx: Arc<Context>, mut req: Request<Body>) -> Result<Response<Body>> {
    dbg!("request");
    let headers = req.headers();
    let key = headers.get(SEC_WEBSOCKET_KEY);
    let derived = key.map(|k| derive_accept_key(k.as_bytes()));
    let ver = req.version();

    smol::spawn(async move {
        dbg!("xd");
        let upgraded = hyper::upgrade::on(&mut req).await
            .expect("upgraded error");
        dbg!("upgraded");

        let client = WebSocketStream::from_raw_socket(
            upgraded.compat(),
            Role::Server,
            None,
        ).await;


        dbg!("taking clients");
        let mut clients = ctx.clients.lock().unwrap();

        clients.insert(1, client);
        dbg!("inserted");

    }).detach();
    
    let mut res = Response::new(Body::empty());
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *res.version_mut() = ver;

    res.headers_mut().append(
        CONNECTION, HeaderValue::from_static("Upgrade")
    );

    res.headers_mut().append(
        UPGRADE, HeaderValue::from_static("websocket")
    );

    res.headers_mut()
        .append(SEC_WEBSOCKET_ACCEPT, derived.unwrap().parse().unwrap());

    Ok(res)
}
