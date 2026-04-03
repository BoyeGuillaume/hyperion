use std::sync::{Arc, Weak};

use crate::{
    HyResult,
    ext::ExtObject,
    instance::{Instance, plugin::Plugin},
    register_plugin,
    schedule::Last,
};
use atomic_enum::atomic_enum;
use bevy_ecs::{prelude::*, system::NonSendMarker};
use chrono::DateTime;
use crossbeam::channel::{Receiver, Sender};
use single_thread_cell::{SingleThreadRefCell, SingleThreadType};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[atomic_enum]
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

#[cfg(feature = "cffi")]
pub mod cffi {
    use crate::{
        api::cffi::r#struct::HyStructureType,
        ext::{ExtObject, ExtObjectCFFIInventory},
        plugin::logger::{LoggerPluginCreateInfo, LoggerRecord},
    };

    use super::LoggerLevel;

    // Basically same as LoggerRecord but responding naming convention for CFFI
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(u32)]
    pub enum HyLoggerLevel {
        Trace,
        Debug,
        Info,
        Warn,
        Error,
    }

    impl From<LoggerLevel> for HyLoggerLevel {
        fn from(level: LoggerLevel) -> Self {
            match level {
                LoggerLevel::Trace => HyLoggerLevel::Trace,
                LoggerLevel::Debug => HyLoggerLevel::Debug,
                LoggerLevel::Info => HyLoggerLevel::Info,
                LoggerLevel::Warn => HyLoggerLevel::Warn,
                LoggerLevel::Error => HyLoggerLevel::Error,
            }
        }
    }

    impl From<HyLoggerLevel> for LoggerLevel {
        fn from(level: HyLoggerLevel) -> Self {
            match level {
                HyLoggerLevel::Trace => LoggerLevel::Trace,
                HyLoggerLevel::Debug => LoggerLevel::Debug,
                HyLoggerLevel::Info => LoggerLevel::Info,
                HyLoggerLevel::Warn => LoggerLevel::Warn,
                HyLoggerLevel::Error => LoggerLevel::Error,
            }
        }
    }

    #[repr(C)]
    pub struct HyLoggerRecord {
        pub timestamp: i64, // Unix timestamp in milliseconds
        pub level: HyLoggerLevel,
        pub p_message: *const std::os::raw::c_char,
        pub p_file: *const std::os::raw::c_char,
        pub line: u32,
        pub p_module_path: *const std::os::raw::c_char,
        pub p_thread_name: *const std::os::raw::c_char,
    }

    impl HyLoggerRecord {
        /// SAFETY: Technically secure, because memory leaks are not a security issue. However,
        /// marked unsafe because called must call CString::from_raw on the string fields to
        /// avoid memory leaks (do this after call of the callback).
        unsafe fn from_logger_record(record: LoggerRecord) -> Self {
            Self {
                timestamp: record.timestamp.timestamp_millis(),
                level: record.level.into(),
                p_message: std::ffi::CString::new(record.message).unwrap().into_raw(),
                p_file: record.file.map_or(std::ptr::null(), |s| {
                    std::ffi::CString::new(s).unwrap().into_raw()
                }),
                line: record.line.unwrap_or(0),
                p_module_path: record.module_path.map_or(std::ptr::null(), |s| {
                    std::ffi::CString::new(s).unwrap().into_raw()
                }),
                p_thread_name: record.thread_name.map_or(std::ptr::null(), |s| {
                    std::ffi::CString::new(s).unwrap().into_raw()
                }),
            }
        }

        /// SAFETY: Caller must call this function on the HyLoggerRecord pointer after the callback
        /// returns to free the C strings and avoid memory leaks.
        unsafe fn destroy(&self) {
            unsafe {
                if !self.p_message.is_null() {
                    let _ =
                        std::ffi::CString::from_raw(self.p_message as *mut std::os::raw::c_char);
                }
                if !self.p_file.is_null() {
                    let _ = std::ffi::CString::from_raw(self.p_file as *mut std::os::raw::c_char);
                }
                if !self.p_module_path.is_null() {
                    let _ = std::ffi::CString::from_raw(
                        self.p_module_path as *mut std::os::raw::c_char,
                    );
                }
                if !self.p_thread_name.is_null() {
                    let _ = std::ffi::CString::from_raw(
                        self.p_thread_name as *mut std::os::raw::c_char,
                    );
                }
            }
        }
    }

    #[allow(non_snake_case)]
    pub type HyLoggerSinkCallback =
        unsafe fn(pRecord: *const HyLoggerRecord, pUserData: *mut std::ffi::c_void);

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct HyLoggerPluginCreateInfo {
        pub s_type: HyStructureType,
        pub level: HyLoggerLevel,
        pub p_sink_callback: HyLoggerSinkCallback,
        pub p_user_data: *mut std::ffi::c_void,
        pub p_next: *mut std::ffi::c_void,
    }

    impl TryFrom<HyLoggerPluginCreateInfo> for LoggerPluginCreateInfo {
        type Error = anyhow::Error;

        fn try_from(value: HyLoggerPluginCreateInfo) -> Result<Self, Self::Error> {
            #[derive(Clone, Copy)]
            pub struct TrustMe(*mut std::ffi::c_void);
            unsafe impl Send for TrustMe {}
            let user_data_trustme = TrustMe(value.p_user_data);

            if value.s_type != HyStructureType::LoggerPluginCreateInfo {
                return Err(anyhow::anyhow!(
                    "Invalid structure type: expected LoggerPluginCreateInfo, got {:?}",
                    value.s_type
                ));
            }

            Ok(LoggerPluginCreateInfo {
                level: value.level.into(),
                sink_callback: Box::new(move |record| {
                    let user_data = user_data_trustme;

                    let cffi_record = unsafe { HyLoggerRecord::from_logger_record(record) };
                    unsafe {
                        (value.p_sink_callback)(
                            &cffi_record as *const HyLoggerRecord as *mut HyLoggerRecord,
                            user_data.0,
                        );
                    }

                    // Don't forget to free the C strings to avoid memory leaks
                    unsafe { cffi_record.destroy() };
                }),
            })
        }
    }

    inventory::submit! {
        ExtObjectCFFIInventory {
            stype: HyStructureType::LoggerPluginCreateInfo as u32,
            callback: |ptr| {
                let create_info = unsafe { *(ptr as *const HyLoggerPluginCreateInfo) };
                let p_next = create_info.p_next;
                let logger_create_info = LoggerPluginCreateInfo::try_from(create_info)?;
                Ok((Box::new(logger_create_info) as Box<dyn ExtObject>, p_next))
            },
        }
    }
}

/// Main logger resource which holds the logger state, including the logger level and the sender/receiver
/// for logging messages from other threads. See [`LoggerStateRes`] for more details.
#[derive(Resource)]
pub struct LoggerStateRes {
    pub level: Arc<AtomicLoggerLevel>,
    sender: Sender<LoggerRecord>,
    receiver: Receiver<LoggerRecord>,
    sink_point: Arc<SingleThreadRefCell<Box<dyn Fn(LoggerRecord) + Send>>>,
}

fn logger_receiver_system(logger_state: Res<LoggerStateRes>, _: NonSendMarker) {
    // Drain the receiver and log all messages
    let sink_point = logger_state.sink_point.borrow_mut();
    while let Ok(msg) = logger_state.receiver.try_recv() {
        if msg.level
            >= logger_state
                .level
                .load(std::sync::atomic::Ordering::Acquire)
        {
            sink_point(msg);
        }
    }
}

/// Retrieve a cross-thread logger callback that can be used outside of systems. This is notably independent from
/// any start linked with bevy-ecs and therefore can be easier to use in some cases.
pub struct Logger {
    sender: Option<(
        Sender<LoggerRecord>,
        Weak<AtomicLoggerLevel>,
        Weak<SingleThreadRefCell<Box<dyn Fn(LoggerRecord) + Send>>>,
    )>,
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

        // Logging statements that are not executed directly on the main thread will be delayed via the sender/receiver
        // to the end of the frame
        let (sender, receiver) = crossbeam::channel::unbounded::<LoggerRecord>();

        // Insert the logger state resource
        instance.world.insert_resource(LoggerStateRes {
            level: Arc::new(AtomicLoggerLevel::new(create_info.level)),
            sender,
            receiver,
            sink_point: Arc::new(SingleThreadRefCell::new(create_info.sink_callback)),
        });
        instance.add_systems(Last, logger_receiver_system);
        Ok(())
    }

    fn is_public(&self) -> bool {
        true
    }
}

register_plugin!(LoggerPlugin);

/// Extension trait for logging.
///
/// This trait provides a convenient `log` method that can be called on various types
pub trait LoggerExt {
    fn log(&self, msg: LoggerRecord);

    fn logger(&self) -> Logger;
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
            if msg.level >= self.level.load(std::sync::atomic::Ordering::Acquire) {
                sink_point(msg);
            }
        } else {
            // Just send it to the receiver to be logged at the end of the frame
            if msg.level >= self.level.load(std::sync::atomic::Ordering::Acquire) {
                self.sender.send(msg)
                    .expect("Failed to send log message to logger system. This likely means the logger system has panicked or is otherwise not running.");
            }
        }
    }

    #[inline]
    fn logger(&self) -> Logger {
        Logger {
            sender: Some((
                self.sender.clone(),
                Arc::downgrade(&self.level),
                Arc::downgrade(&self.sink_point),
            )),
        }
    }
}

impl<'a> LoggerExt for Res<'a, LoggerStateRes> {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        self.as_ref().log(msg);
    }

    #[inline]
    fn logger(&self) -> Logger {
        self.as_ref().logger()
    }
}

impl<T: LoggerExt> LoggerExt for Option<T> {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        if let Some(logger) = self.as_ref() {
            logger.log(msg);
        }
    }

    #[inline]
    fn logger(&self) -> Logger {
        if let Some(logger) = self.as_ref() {
            logger.logger()
        } else {
            Logger { sender: None }
        }
    }
}

impl<T: LoggerExt> LoggerExt for &T {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        (*self).log(msg);
    }

    #[inline]
    fn logger(&self) -> Logger {
        (*self).logger()
    }
}

impl<T: LoggerExt> LoggerExt for &mut T {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        (*self as &T).log(msg);
    }

    #[inline]
    fn logger(&self) -> Logger {
        (*self as &T).logger()
    }
}

impl LoggerExt for World {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        self.get_resource::<LoggerStateRes>().log(msg);
    }

    #[inline]
    fn logger(&self) -> Logger {
        self.get_resource::<LoggerStateRes>().logger()
    }
}

impl LoggerExt for Instance {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        self.world.log(msg);
    }

    #[inline]
    fn logger(&self) -> Logger {
        self.world.logger()
    }
}

impl LoggerExt for Logger {
    #[inline]
    fn log(&self, msg: LoggerRecord) {
        if let Some((sender, atomic_level, sink_point)) = &self.sender {
            // Attempt to update atomic_level/sink_point
            let atomic_level = atomic_level.upgrade();
            let sink_point = sink_point.upgrade();
            if atomic_level.is_none() || sink_point.is_none() {
                // The logger system has likely been dropped, so we can't log anymore. Just return silently.
                return;
            }

            let atomic_level = atomic_level.unwrap();
            let sink_point = sink_point.unwrap();

            // Check the log level before sending to avoid unnecessary cross-thread communication
            if msg.level < atomic_level.load(std::sync::atomic::Ordering::Acquire) {
                return;
            }
            if sink_point.check_same_thread() {
                let sink_point = sink_point.borrow_mut();
                sink_point(msg);
            } else {
                sender.send(msg)
                    .expect("Failed to send log message to logger system. This likely means the logger system has panicked or is otherwise not running.");
            }
        }
    }

    #[inline]
    fn logger(&self) -> Logger {
        Logger {
            sender: self.sender.clone(),
        }
    }
}

/// Logger macro exposed to plugins
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
