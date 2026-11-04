use worker::*;
use serde::{Deserialize, Serialize};

pub mod crdt;
use crdt::{SpatialRegistry, SpatialEntityState};

#[durable_object]
pub struct SpatialStateDO {
    state: State,
    env: Env,
    registry: SpatialRegistry,
}

#[durable_object]
impl DurableObject for SpatialStateDO {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            registry: SpatialRegistry::new(),
        }
    }

    async fn fetch(&mut self, mut req: Request) -> Result<Response> {
        let path = req.path();

        if path.ends_with("/sync") {
            let entity_state: SpatialEntityState = match req.json().deserialize().await {
                Ok(s) => s,
                Err(e) => return Response::error(format!("Bad Request: {}", e), 400),
            };

            match self.registry.apply_delta(entity_state) {
                Ok(_) => Response::ok("State delta verified, accepted, and persisted in Durable Object"),
                Err(err) => Response::error(format!("Rejection: {}", err), 403),
            }
        } else if path.ends_with("/state") {
            Response::from_json(&self.registry)
        } else {
            Response::error("Not Found", 404)
        }
    }
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .post_async("/api/spatial/:id/sync", |mut req, ctx| async move {
            let id_str = ctx.param("id").unwrap();
            let namespace = ctx.durable_object("SPATIAL_DO")?;
            let id = namespace.id_from_name(&id_str)?;
            let stub = id.get_stub()?;
            stub.fetch_with_request(req).await
        })
        .get("/health", |_, _| {
            Response::ok("HOARE.ai Spatial Engine - Durable Object Edge Node Active")
        })
        .run(req, env)
        .await
}
