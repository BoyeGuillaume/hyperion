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
                    %current.acc: i32 = phi [0i32, entry], [%next.acc, loop_body]
                    %is_zero: i1 = icmp.eq %current.b, 0i32
                    branch %is_zero, loop_end, loop_body

                loop_body:
                    %next.acc: i32 = imul.wrap %current.acc, %a
                    %next.b: i32 = isub.wrap %current.b, 1i32
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
source_code = b'\x7fHYMODIR=0.1.2\x00(\xb5/\xfd\x00X\x05\x06\x00RG\x1b(\x80K\xd2\x0130\x13\xc6$\xb9\x19\xe7\x00\x9cK\xcchi\xbe\x0b\xa5S\x0f\x04L 0\x17\xaa\x02\x8c\xed\xaf\xc0\x0c!\x84l\x99\x02\x7f\xbf\xc9\xfa\xea\xb2\xf5"H\x9as\x1b.\x8b^f\xf4\xb3\xbf\xed\xad_\x08\x18\xea\x0c\x18\x81\xc2y\xa9\xac\x9f\x15\xb098>\x8b\x84g\xa2\x00i$\x8e\xa5\xf1\xf7?R\xa6LD\x82T\x05R\xc5\x07\xa4\xac8a\xe0/\xf8\x1b! @\x02G\x0c\xc9\x03F2\x80\x95\x81.]w\x16\x88\xf0\x03^#\xafa\xf6\x0e2J\x06c\x8a\x88\xd1\xa0\x05\x9a\xe4\x06b\x0f3\xa9\x96\xd0\x13\xe0\x06\xa1`z\x86\'\xdcp\xadX:f\xc2N%\x98\xa2\xd4\x03\xce0\xc9HD\xe2L\xaf\x18\xc1\x01\xd1 \x0b\n'

# Use the compiled module to perform computations
module = api.load_module(instance, source_code)
print(module)

del instance
