use crate::{
    config, topology, signal,
};
use tokio::{
    sync::mpsc,
};

pub struct ApplicationConfig {
    pub config_paths: Vec<config::ConfigPath>,
    pub topology: topology,
    pub graceful_crash: mpsc::UnboundedReceiver<()>,
    pub signal_handler: signal::SignalHandler,
    pub signal_rx: signal::SignalRx,
}

impl ApplicationConfig {

}