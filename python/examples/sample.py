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
    node_id=42,
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
source_code = b"\x7fHYMODIR=0.1.1\x00(\xb5/\xfd\x00X\xd5\x05\x00\x12\xc7\x19%\x80K\xd2\x013\x03F<#\xd2\xc2r\xa1\x88i\xa3\n\x1b\xeao'a&\x0e4\x15\x9cR\xb6\xc26\xb5\x1a!d\xcb\x14\x7f\xbf\xb9\xea\xb4%+\xb5\xb3\xb0\xd6.\xdc\x059M\xe8_\x7f;\xeb\x14\x03V.\xd1\x0b\x10\xce\xc7\x14\xffU\x84\xa4\x02d\xe23\xd1D\x82\xcd\x81Ra\xfe\xc2\x86\x8b,\x0cOr(&\x98xR\x84\x0e\xf2\xfe\xc1\xdf! @\x02G,\xab\x03Fb\xc0J\x83\xee\nw&\x10\x85\x07\xbcF\x0e\x86\xd99\xc8\x88\x0c\x06\x14\x01\xa3\xb9\x05\xfa\xc9]\xc4\x1eeR!\xa1-\x80\x87\x9e`Z\x86\xa7\xdcp\xaaX;\xc6\xf9\xc4\x8e\x00\x14\xa7r\x84F2\x92H\x1c\xd1\x0bFh@0\x88\x80\x02"

# Use the compiled module to perform computations
module = api.load_module(instance, source_code)
print(module)

del instance
