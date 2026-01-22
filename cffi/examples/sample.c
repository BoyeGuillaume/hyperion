#include <hycore.h>
#include <stdio.h>

static const char *hycore_c_str =
    "define i32 square(%a: i32) {\n"
    "entry:\n"
    "  %result: i32 = imul.wrap %a, %a\n"
    "  ret %result\n"
    "}\n";

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

void callback_function(struct HyLogMessageEXT *message);
void print_hex_ascii(const uint8_t *data, uint32_t length);

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

    /* Compile a simple module */
    HyModuleSourceInfo sourceInfo;
    sourceInfo.sType = HY_STRUCTURE_TYPE_MODULE_SOURCE_INFO;
    sourceInfo.sourceType = HY_MODULE_SOURCE_TYPE_ASSEMBLY;
    sourceInfo.filename = "sample.c";
    sourceInfo.data = (const uint8_t *)hycore_c_str;

    const HyModuleSourceInfo *sources[] = {&sourceInfo};
    HyModuleCompileInfo compileInfo;
    compileInfo.sType = HY_STRUCTURE_TYPE_MODULE_COMPILE_INFO;
    compileInfo.ppSources = sources;
    compileInfo.sourcesCount = sizeof(sources) / sizeof(sources[0]);

    uint8_t *compiledData = NULL;
    uint32_t compiledDataLen = 0;
    result = hyCompileModule(instance, &compileInfo, &compiledData, &compiledDataLen);
    if (result != HY_RESULT_SUCCESS)
    {
        printf("Module compilation failed. Error code: %d\n", result);
        hyDestroyInstance(instance);
        return -1;
    }

    printf("Module compiled successfully. Compiled data length: %u bytes\n", compiledDataLen);
    putchar('\n');
    printf("Compiled Module Data (Hex):\n");
    print_hex_ascii(compiledData, compiledDataLen);
    putchar('\n');

    /* free compiled data if necessary */
    free(compiledData);

    /* Display compiled data in hexadecimal format */

    /* Clean up and exit */
    hyDestroyInstance(instance);
    printf("Hycore instance destroyed.\n");

    return 0;
}

void print_hex_ascii(const uint8_t *data, uint32_t length)
{
    uint32_t offset = 0;
    while (offset < length)
    {
        printf("%08X | ", offset); /* offset */
        for (uint32_t i = 0; i < 16; i++)
        {
            if (offset + i < length)
                printf("%02X ", data[offset + i]);
            else
                printf("   ");
        }
        printf("| ");
        for (uint32_t i = 0; i < 16; i++)
        {
            if (offset + i < length)
            {
                char c = data[offset + i];
                if (c >= 32 && c <= 126)
                    printf("%c", c);
                else
                    printf(".");
            }
        }
        printf("\n");

        offset += 16;
    }
}

void callback_function(struct HyLogMessageEXT *message)
{
    printf("[%s][%s:%u] -- %s\n", log_level_to_string(message->level), message->file, message->line, message->message);
}