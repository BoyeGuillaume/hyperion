use crate::instance::plugin::Plugin;
use bevy_ecs::prelude::*;

#[derive(Resource)]
pub struct TokioRuntime(tokio::runtime::Runtime);

impl std::ops::Deref for TokioRuntime {
    type Target = tokio::runtime::Runtime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for TokioRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct TokioPlugin;

impl Plugin for TokioPlugin {
    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        _ext: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(
                std::thread::available_parallelism()
                    .map(|x| {
                        let x = x.get();
                        if x <= 3 { 1 } else { (x << 1).min(3) }
                    })
                    .unwrap_or(2),
            )
            .thread_name("tokio-thread")
            .enable_all()
            .build()?;

        // Insert the Tokio runtime as a resource in the Bevy ECS world
        instance.insert_resource(TokioRuntime(rt));

        Ok(())
    }
}
