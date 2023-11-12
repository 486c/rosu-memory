use tide::Request;
use std::sync::Arc;
use tide_websockets::{Message, WebSocket};
use futures_lite::future;

use crate::structs::Context;

pub async fn websocket_handle(ctx: Arc<Context>) {
    let _span = tracy_client::span!("websocket loop");

    let values = ctx.values.lock().unwrap();
    let serizalized_values = serde_json::to_string(&(*values))
        .unwrap();
    drop(values);

    let mut clients = ctx.clients.lock().unwrap();

    clients.retain(|_client_id, websocket| {
        future::block_on(async {
            let _ = websocket.send(
                Message::Text(
                    serizalized_values.clone()
                    )
                ).await;

            true
        })
    });

    drop(clients);
}

pub fn server_thread(
    ctx: Arc<Context>
) {
    tracy_client::set_thread_name!("server thread");
    let mut app = tide::with_state(ctx.clone());

    app.at("/")
        .with(WebSocket::new(|req: Request<Arc<Context>>, stream| async move {
            let _span = tracy_client::span!("websocket connection");
            let ctx = req.state();

            let mut clients = ctx.clients.lock().unwrap();
            clients.insert(1, stream);

            Ok(())
        }))
    .get(|_| async move { 
        Ok("not a websocket request! ")
    });

    let server = app.listen("127.0.0.1:9001");

    future::block_on(async {
        server.await.unwrap();
    })
}
