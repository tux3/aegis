//! Admin handlers are authenticated. They are reachable only by REST, not by websocket.

mod handler_inventory;
pub use handler_inventory::admin_handler_iter;
