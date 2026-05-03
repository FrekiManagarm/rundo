pub mod handlers;
pub mod registry;
pub mod room;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::create_room).get(handlers::list_rooms))
        .route("/{id}", get(handlers::get_room).delete(handlers::delete_room))
        .route("/{id}/join", get(crate::signaling::handler::ws_handler))
}
