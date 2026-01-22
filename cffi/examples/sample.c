#include <hycore.h>
#include <stdio.h>

static const char *log_level_to_string(HyLogLevelEXT level)
{
    switch (level)
    {
    case HY_LOG_LEVEL_TRACE:
        return "TRACE";
    case HY_LOG_LEVEL_DEBUG:
        return "DEBUG";
    case HY_LOG_LEVEL_INFO:
        return "INFO";
    case HY_LOG_LEVEL_WARN:
        return "WARN";
    case HY_LOG_LEVEL_ERROR:
        return "ERROR";
    default:
        return "UNKNOWN";
    }
}

void callback_function(struct HyLogMessageEXT *message)
{
    printf("[%s][%s:%u] -- %s\n", log_level_to_string(message->level), message->file, message->line, message->message);
}

int main()
{
    /* Retrieve and print Hycore version information */
    HyVersionInfo version;
    hyGetVersionInfo(&version);
    printf("Hycore Version: %u.%u.%u\n", version.major, version.minor, version.patch);

    /* Construct a new instance */
    HyApplicationInfo appInfo;
    appInfo.sType = HY_STRUCTURE_TYPE_APPLICATION_INFO;
    appInfo.applicationVersion = version;
    appInfo.pApplicationName = "SimpleCApp";
    appInfo.engineVersion = version;
    appInfo.pEngineName = "HycoreEngine";

    HyLogCreateInfoEXT logCreateInfo;
    logCreateInfo.sType = HY_STRUCTURE_TYPE_LOG_CREATE_INFO_EXT;
    logCreateInfo.callback = callback_function;
    logCreateInfo.pNext = NULL;

    const char *extensions[] = {HY_LOGGER_NAME_EXT};
    HyInstanceCreateInfo createInfo;
    createInfo.sType = HY_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    createInfo.pApplicationInfo = &appInfo;
    createInfo.ppEnabledExtensions = extensions;
    createInfo.enabledExtensionsCount = sizeof(extensions) / sizeof(extensions[0]);
    createInfo.nodeId = 0;
    createInfo.pNext = &logCreateInfo;

    HyInstance *instance;
    HyResult result = hyCreateInstance(&createInfo, &instance);
    if (result != HY_RESULT_SUCCESS)
    {
        printf("Failed to create Hycore instance. Error code: %d\n", result);
        return -1;
    }

    printf("Hycore instance created successfully.\n");
    printf("Instance pointer: %p\n", (void *)instance);

    /* Clean up and exit */
    hyDestroyInstance(instance);
    printf("Hycore instance destroyed.\n");

    return 0;
}