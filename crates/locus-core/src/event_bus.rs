//! Event bus: broadcast channel for RuntimeEvent

use tokio::sync::broadcast;

use crate::events::RuntimeEvent;

pub type EventTx = broadcast::Sender<RuntimeEvent>;
pub type EventRx = broadcast::Receiver<RuntimeEvent>;

const CAPACITY: usize = 64;

pub fn event_bus() -> (EventTx, EventRx) {
    broadcast::channel(CAPACITY)
}
