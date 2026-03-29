use crate::instance::{
    analysis::{
        cfg::DeriveCfgPlugin, destmap::DeriveDestMapPlugin, loops::DeriveLoopAnalysisPlugin,
    },
    plugin::Plugin,
};

pub mod cfg;
pub mod destmap;
pub mod loops;

pub struct AnalysisPlugin;
impl Plugin for AnalysisPlugin {
    fn init(
        &mut self,
        instance: &mut super::Instance,
        _ext: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        instance.add_plugin(DeriveCfgPlugin)?;
        instance.add_plugin(DeriveDestMapPlugin)?;
        instance.add_plugin(DeriveLoopAnalysisPlugin)?;
        Ok(())
    }
}
