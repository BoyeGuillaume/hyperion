import hypi.api as api

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
            level=0,
        )
    ]
)

instance = api.create_instance(instance_create_info)
print(f"Created instance: {instance}")
