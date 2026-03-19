pub mod api;
pub mod ext;
pub mod instance;
pub mod module;
pub mod resource;

pub extern crate bevy_ecs;
pub extern crate inventory;
pub extern crate smallbox;

pub type HyError = anyhow::Error;
pub type HyResult<T> = Result<T, HyError>;
