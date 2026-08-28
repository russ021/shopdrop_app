use actix::prelude::*;
use actix_web_actors::ws;
use tokio::sync::broadcast;

pub struct WsSession {
    pub rx: broadcast::Receiver<String>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        let mut rx = self.rx.resubscribe();

        // Bridge Tokio's broadcast channel into the Actix actor mailbox. Each
        // connection gets its own receiver, so clients do not consume one
        // another's updates.
        actix_rt::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        addr.do_send(ServerMessage(msg));
                    }
                    Err(e) => {
                        log::error!("Broadcast receiver error: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ServerMessage(pub String);

impl Handler<ServerMessage> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => (),
            Ok(ws::Message::Text(_)) => (),
            Ok(ws::Message::Binary(_)) => (),
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            }
            Err(e) => {
                log::error!("WebSocket stream error: {}", e);
                ctx.stop();
            }
            _ => (),
        }
    }
}