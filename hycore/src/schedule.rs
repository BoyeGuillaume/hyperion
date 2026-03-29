use std::any::Any;

use bevy_ecs::{
    prelude::*,
    schedule::{ExecutorKind, InternedScheduleLabel, ScheduleLabel},
};

use crate::{hydebug, hytrace, instance::plugin::Plugin};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub(crate) struct MainStartup;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub(crate) struct Main;

fn main_startup_system(world: &mut World, mut run_at_least_once: Local<bool>) {
    if *run_at_least_once {
        return;
    }
    *run_at_least_once = true;

    // Run startup schedules in order
    world.resource_scope(|world, schedule_order: Mut<ScheduleOrder>| {
        for label in &schedule_order.startup_labels.list {
            hydebug!(world;
                "Running startup schedule {:?}",
                label
            );
            let _ = world.try_run_schedule(*label);
            world.flush();
        }
    });
}

/// Runs once, before [`Startup`]. See [`Startup`] for more details.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PreStartup;

/// Runs once, after [`Startup`]. See [`Startup`] for more details.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostStartup;

/// Runs once, when the [`crate::instance::Instance`] is being initialized.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Startup;

/// Planning phase where we determine what functions/elements to focus on when optimising.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Plan;

/// Runs every frame, perform update duty. Depends on [`Plan`] to determine what to run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Update;

/// Runs once every frame, perform post update duty. Notably, this is where all analysis
/// that should be performed (outside of repeatable updating) should be performed.
///
/// Things such as CFG construction, call graph detection...
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostUpdate;

/// Last schedule to run every frame.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Last;

/// Runs once, when the [`crate::instance::Instance`] is being dropped. This is where all plugins should perform their cleanup.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct DropSchedule;

/// A list of schedule labels, used for defining the order of schedules. See [`ScheduleOrder`] for more details.
#[derive(Debug)]
pub struct ScheduleOrderList {
    pub list: Vec<InternedScheduleLabel>,
}

fn main_schedule_system(world: &mut World) {
    world.resource_scope(|world, schedule_order: Mut<ScheduleOrder>| {
        for label in &schedule_order.main_labels.list {
            if label.0.type_id() == Update.type_id() {
                for update_schedule_label in &schedule_order.udpate_labels.list {
                    hytrace!(world;
                        "Running update schedule {:?}",
                        update_schedule_label
                    );
                    let _ = world.try_run_schedule(*update_schedule_label);
                    world.flush();
                }
            } else {
                let _ = world.try_run_schedule(*label);
                world.flush();
            }
        }
    });
}

impl From<Vec<InternedScheduleLabel>> for ScheduleOrderList {
    fn from(value: Vec<InternedScheduleLabel>) -> Self {
        Self { list: value }
    }
}

impl ScheduleOrderList {
    pub fn insert_before(&mut self, before: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .list
            .iter()
            .position(|label| *label == before.intern())
            .unwrap_or_else(|| {
                panic!(
                    "Schedule label {:?} not found in schedule order list",
                    before.intern()
                )
            });
        self.list.insert(index, schedule.intern());
    }

    pub fn insert_after(&mut self, after: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .list
            .iter()
            .position(|label| *label == after.intern())
            .unwrap_or_else(|| {
                panic!(
                    "Schedule label {:?} not found in schedule order list",
                    after.intern()
                )
            });
        self.list.insert(index + 1, schedule.intern());
    }
}

/// Defines the schedule ordering
#[derive(Resource, Debug)]
pub(super) struct ScheduleOrder {
    /// Instance startup schedule order list.
    pub startup_labels: ScheduleOrderList,

    /// Instance main schedule order list.
    pub main_labels: ScheduleOrderList,

    /// Instance update schedule order list. This is the order in which the update schedules will be run every frame (can run multiple times per frame).
    pub udpate_labels: ScheduleOrderList,
}

impl Default for ScheduleOrder {
    fn default() -> Self {
        Self {
            startup_labels: vec![PreStartup.intern(), Startup.intern(), PostStartup.intern()]
                .into(),
            main_labels: vec![
                Plan.intern(),
                Update.intern(),
                PostUpdate.intern(),
                Last.intern(),
            ]
            .into(),
            udpate_labels: vec![Update.intern()].into(),
        }
    }
}

// Plugin to initialize the schedule order resource
pub struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        _: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        let mut setup_schedule = Schedule::new(MainStartup);
        setup_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        let mut main_schedule = Schedule::new(Main);
        main_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        let mut drop_schedule = Schedule::new(DropSchedule);
        drop_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        instance.add_schedule(setup_schedule);
        instance.add_systems(MainStartup, main_startup_system);

        instance.add_schedule(main_schedule);
        instance.add_systems(Main, main_schedule_system);

        instance.add_schedule(drop_schedule);
        instance.insert_resource(ScheduleOrder::default());
        Ok(())
    }
}
