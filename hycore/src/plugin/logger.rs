use crate::{
    HyResult,
    ext::ExtObject,
    instance::{Instance, plugin::Plugin},
    register_plugin,
    schedule::Last,
};
use bevy_ecs::{prelude::*, system::NonSendMarker};
use chrono::DateTime;
use crossbeam::channel::{Receiver, Sender};
use single_thread_cell::{SingleThreadRefCell, SingleThreadType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum LoggerLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LoggerLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggerLevel::Trace => write!(f, "TRACE"),
            LoggerLevel::Debug => write!(f, "DEBUG"),
            LoggerLevel::Info => write!(f, "INFO"),
            LoggerLevel::Warn => write!(f, "WARN"),
            LoggerLevel::Error => write!(f, "ERROR"),
        }
    }
}

pub struct LoggerRecord {
    pub timestamp: DateTime<chrono::Local>,
    pub level: LoggerLevel,
    pub message: String,
    pub file: Option<&'static str>,
    pub line: Option<u32>,
    pub module_path: Option<&'static str>,
    pub thread_id: std::thread::ThreadId,
    pub thread_name: Option<String>,
}

pub struct LoggerPluginCreateInfo {
    pub level: LoggerLevel,
    pub sink_callback: Box<dyn Fn(LoggerRecord) + Send>,
}

impl std::fmt::Debug for LoggerPluginCreateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggerPluginCreateInfo")
            .field("level", &self.level)
            .finish_non_exhaustive()
    }
}

impl ExtObject for LoggerPluginCreateInfo {}

/// Main logger resource which holds the logger state, including the logger level and the sender/receiver
/// for logging messages from other threads. See [`LoggerStateRes`] for more details.
#[derive(Resource)]
pub struct LoggerStateRes {
    pub level: LoggerLevel,
    sender: Sender<LoggerRecord>,
    receiver: Receiver<LoggerRecord>,
    sink_point: SingleThreadRefCell<Box<dyn Fn(LoggerRecord) + Send>>,
}

fn logger_receiver_system(logger_state: Res<LoggerStateRes>, _: NonSendMarker) {
    // Drain the receiver and log all messages
    let sink_point = logger_state.sink_point.borrow_mut();
    while let Ok(msg) = logger_state.receiver.try_recv() {
        sink_point(msg);
    }
}

/// Logger plugin, which provides logging capabilities to the instance. See [`LoggerPluginCreateInfo`] for the configuration of the plugin.
#[derive(Default)]
pub struct LoggerPlugin;

impl Plugin for LoggerPlugin {
    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        ext_list: Option<&mut crate::ext::ExtList>,
    ) -> HyResult<()> {
        let ext_list = ext_list.unwrap();
        let create_info = ext_list
            .pop::<LoggerPluginCreateInfo>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "LoggerPluginCreateInfo object not found in extension list. LoggerPlugin requires a LoggerPluginCreateInfo ext object to be constructed."
                )
            })?;

        // Logging statement that are not executed directly on the main thread will be delayed via the sender/receiver
        // to the end of the frame
        let (sender, receiver) = crossbeam::channel::unbounded::<LoggerRecord>();

        // Insert the logger state resource
        instance.world.insert_resource(LoggerStateRes {
            level: create_info.level,
            sender,
            receiver,
            sink_point: SingleThreadRefCell::new(create_info.sink_callback),
        });
        instance.add_systems(Last, logger_receiver_system);
        Ok(())
    }

    fn is_public(&self) -> bool {
        true
    }
}

register_plugin!(LoggerPlugin);

pub trait LoggerExt {
    fn log(&self, msg: LoggerRecord);
}

impl LoggerExt for LoggerStateRes {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        // Log it
        if self.sink_point.check_same_thread() {
            let sink_point = self.sink_point.borrow_mut();

            // 1. Empty the receiver queue first to ensure that all messages are logged in order
            while let Ok(queued_msg) = self.receiver.try_recv() {
                sink_point(queued_msg);
            }

            // 2. Log the current message
            if msg.level >= self.level {
                sink_point(msg);
            }
        } else {
            // Just send it to the receiver to be logged at the end of the frame
            if msg.level >= self.level {
                self.sender.send(msg)
                    .expect("Failed to send log message to logger system. This likely means the logger system has panicked or is otherwise not running.");
            }
        }
    }
}

impl<T: LoggerExt> LoggerExt for Option<T> {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        if let Some(logger) = self.as_ref() {
            logger.log(msg);
        }
    }
}

impl<T: LoggerExt> LoggerExt for &T {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        (*self).log(msg);
    }
}

impl<T: LoggerExt> LoggerExt for &mut T {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        (*self as &T).log(msg);
    }
}

impl LoggerExt for World {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        self.get_resource::<LoggerStateRes>().log(msg);
    }
}

impl LoggerExt for Instance {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        self.world.log(msg);
    }
}

/// Logger macro
#[macro_export]
macro_rules! hylog {
    (
        $logger_state:expr;
        $level:expr,
        $fmt:expr $(, $($arg:tt)+)?
    ) => {
        crate::plugin::logger::LoggerExt::log(&$logger_state, crate::plugin::logger::LoggerRecord {
            timestamp: chrono::Local::now(),
            level: $level,
            message: format!($fmt $(, $($arg)+)?),
            file: Some(file!()),
            line: Some(line!()),
            module_path: Some(module_path!()),
            thread_id: std::thread::current().id(),
            thread_name: std::thread::current().name().map(|s| s.to_string()),
        });
    };
}

#[macro_export]
macro_rules! hytrace {
    (
        $logger_state:expr;
        $fmt:expr $(, $($arg:tt)+)?
    ) => {
        crate::hylog!($logger_state; crate::plugin::logger::LoggerLevel::Trace, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! hydebug {
    (
        $logger_state:expr;
        $fmt:expr $(, $($arg:tt)+)?
    ) => {
        crate::hylog!($logger_state; crate::plugin::logger::LoggerLevel::Debug, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! hyinfo {
    (
        $logger_state:expr;
        $fmt:expr $(, $($arg:tt)+)?
    ) => {
        crate::hylog!($logger_state; crate::plugin::logger::LoggerLevel::Info, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! hywarn {
    (
        $logger_state:expr;
        $fmt:expr $(, $($arg:tt)+)?
    ) => {
        crate::hylog!($logger_state; crate::plugin::logger::LoggerLevel::Warn, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! hyerror {
    (
        $logger_state:expr;
        $fmt:expr $(, $($arg:tt)+)?
    ) => {
        crate::hylog!($logger_state; crate::plugin::logger::LoggerLevel::Error, $fmt $(, $($arg)+)?);
    };
}
