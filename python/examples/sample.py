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
source_code = api.compile_module(
    instance,
    api.ModuleCompileInfo(
        sources=[
            api.ModuleSourceInfo(
                source_type=api.ModuleSourceType.ASSEMBLY,
                filename="example_module.hyasm",
                data="""
                ; Example Hyperion assembly module
                define i32 pow(%a: i32, %b: i32) {
                entry:
                    jump loop_check

                loop_check:
                    %current.b: i32 = phi [%b, entry], [%next.b, loop_body]
                    %current.acc: i32 = phi [i32 0, entry], [%next.acc, loop_body]
                    %is_zero: i1 = icmp.eq %current.b, i32 0
                    branch %is_zero, loop_end, loop_body

                loop_body:
                    %next.acc: i32 = imul.wrap %current.acc, %a
                    %next.b: i32 = isub.wrap %current.b, i32 1
                    jump loop_check

                loop_end:
                    ret %current.acc
                }
                """,
            )
        ]
    )
)

print(source_code)

del instance
