import hypi.api as api

def log_callback(log_message):
    print(
        f"{log_message.timepoint.strftime('%Y-%m-%d %H:%M:%S'):<20} "
        f"[{log_message.level.name:<6}] "
        f"{log_message.module:>15} -- "
        f"{log_message.message}"
    )

application_info = api.ApplicationInfo(
    application_name="python_example",
    application_version=api.Version(1, 0, 0),
    engine_name="engine_name",
    engine_version=api.Version(0, 1, 0),
)

instance_create_info = api.InstanceCreateInfo(
    application_info=application_info,
    enabled_extensions=[
        api.InstanceEXT.LOGGER,
    ],
    ext=[
        api.LogCreateInfoEXT(
            level=api.LogLevelEXT.TRACE,
            callback=log_callback,
        )
    ]
)

instance = api.create_instance(instance_create_info)
del instance
