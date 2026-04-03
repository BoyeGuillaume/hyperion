pub mod api;
pub mod ext;
pub mod hyir;
pub mod instance;
pub mod plugin;
pub mod resource;
pub mod schedule;
pub mod tokio;
// pub mod task_pool;

pub extern crate bevy_ecs;
pub extern crate chrono;
pub extern crate inventory;
pub extern crate smallbox;

pub type HyError = anyhow::Error;
pub type HyResult<T> = Result<T, HyError>;
