use std::time::{Duration, Instant};

use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web_actors::ws;
use futures::Future;

use crate::{
    db::models,
    services::system::{Subscribe, SystemMonitor, Unsubscribe},
};

/// How frequently we send heartbeats to the client
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// Maximum time we'll wait for a ping from the client before timing out
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Ws {
    system_monitor: Addr<SystemMonitor>,
    subscriber_id: Option<usize>,

    /// Client must send ping at least once per CLIENT_TIMEOUT
    last_heartbeat: Instant,
}

impl Actor for Ws {
    type Context = ws::WebsocketContext<Self>;

    /// Start the heartbeat process on actor start
    fn started(&mut self, ctx: &mut Self::Context) {
        // subscribe to system updates
        self.system_monitor
            .send(Subscribe(Addr::recipient(ctx.address())))
            .map(|id| self.subscriber_id = Some(id))
            .wait()
            .unwrap();

        self.heartbeat(ctx);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for Ws {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.last_heartbeat = Instant::now();
            }
            ws::Message::Text(text) => ctx.text(text),
            ws::Message::Binary(bin) => ctx.binary(bin),
            ws::Message::Close(_) => self.disconnect(ctx),
            ws::Message::Nop => (),
        }
    }
}

impl Ws {
    pub fn new(system_monitor: Addr<SystemMonitor>) -> Self {
        Self {
            system_monitor,
            subscriber_id: None,
            last_heartbeat: Instant::now(),
        }
    }

    /// Send ping to client every second and determine whether we've
    /// timed out
    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |this, ctx| {
            if Instant::now().duration_since(this.last_heartbeat)
                > CLIENT_TIMEOUT
            {
                log::warn!("Websocket Client heartbeat failed, disconnecting");
                this.disconnect(ctx);
                return;
            }

            ctx.ping("");
        });
    }

    /// Send system status updates to the client
    fn send_update(
        &self,
        update: models::DiskUsage,
        ctx: &mut <Self as Actor>::Context,
    ) {
        ctx.text(update)
    }

    fn disconnect(&self, ctx: &mut <Self as Actor>::Context) {
        if let Some(id) = self.subscriber_id {
            self.system_monitor.do_send(Unsubscribe(id));
        }
        ctx.stop();
    }
}

impl Handler<models::DiskUsage> for Ws {
    type Result = ();

    fn handle(&mut self, update: models::DiskUsage, ctx: &mut Self::Context) {
        self.send_update(update, ctx)
    }
}
