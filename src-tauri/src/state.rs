use std::sync::Mutex;

use crate::gateway::Gateway;
use crate::store::Store;

pub struct AppState {
    pub store: Mutex<Store>,
    pub gateway: Mutex<Gateway>,
}
