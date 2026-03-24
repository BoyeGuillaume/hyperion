use bevy_ecs::{
    prelude::*,
    schedule::{ExecutorKind, InternedScheduleLabel, ScheduleLabel},
};

use crate::{hydebug, instance::plugin::Plugin};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub(crate) struct MainStartup;

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

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub(crate) struct Main;

/// Update the main schedule, which runs every frame. See [`Main`] for more details.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Update;

/// Runs every frame, perform post update duty.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostUpdate;

/// Runs every frame at the very last. This is where all executed are guaranteed to have been executed
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
            world.run_schedule(*label);
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
}

impl Default for ScheduleOrder {
    fn default() -> Self {
        Self {
            startup_labels: vec![PreStartup.intern(), Startup.intern(), PostStartup.intern()]
                .into(),
            main_labels: vec![Update.intern(), PostUpdate.intern(), Last.intern()].into(),
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
