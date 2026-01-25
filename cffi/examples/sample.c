#include <hycore.h>
#include <stdio.h>
#include <math.h>

static const char *hycore_c_str =
    "define i32 square(%a: i32) {\n"
    "entry:\n"
    "  %result: i32 = imul.wrap %a, %a\n"
    "  ret %result\n"
    "}\n";

#if defined(_MSC_VER)
#define COLOR_RESET ""
#define COLOR_RED ""
#define COLOR_GREEN ""
#define COLOR_YELLOW ""
#define COLOR_BLUE ""
#define COLOR_BRIGHT_BLACK ""
#else
#define COLOR_RESET "\x1b[0m"
#define COLOR_RED "\x1b[31m"
#define COLOR_GREEN "\x1b[32m"
#define COLOR_YELLOW "\x1b[33m"
#define COLOR_BLUE "\x1b[34m"
#define COLOR_BRIGHT_BLACK "\x1b[90m"
#endif

static const char *log_level_to_color(HyLogLevelEXT level)
{
    switch (level)
    {
    case HY_LOG_LEVEL_TRACE:
        return COLOR_BRIGHT_BLACK;
    case HY_LOG_LEVEL_DEBUG:
        return COLOR_BLUE;
    case HY_LOG_LEVEL_INFO:
        return COLOR_GREEN;
    case HY_LOG_LEVEL_WARN:
        return COLOR_YELLOW;
    case HY_LOG_LEVEL_ERROR:
        return COLOR_RED;
    default:
        return COLOR_RESET;
    }
}

static const char *log_level_to_string(HyLogLevelEXT level)
{
    switch (level)
    {
    case HY_LOG_LEVEL_TRACE:
        return "[TRACE]";
    case HY_LOG_LEVEL_DEBUG:
        return "[DEBUG ]";
    case HY_LOG_LEVEL_INFO:
        return "[INFO  ]";
    case HY_LOG_LEVEL_WARN:
        return "[WARN  ]";
    case HY_LOG_LEVEL_ERROR:
        return "[ERROR]";
    default:
        return "[UNKNOWN]";
    }
}

void callback_function(struct HyLogMessageEXT *message);
void print_hex_ascii(const uint8_t *data, uint32_t length, bool compute_stats);

int main(int argc, char **argv)
{
    if (argc < 1 || argc >= 3)
    {
        printf("Usage: %s <optional_assembly_file>\n", argv[0]);
        return -1;
    }

    /* Read the assembly file if provided, overwise default to hycore_c_str */
    const char *assembly_data = hycore_c_str;
    bool assembly_data_allocated = false;

    if (argc == 2)
    {
        const char *filename = argv[1];
        FILE *file = fopen(filename, "rb");
        if (!file)
        {
            printf("Failed to open file: %s\n", filename);
            return -1;
        }
        fseek(file, 0, SEEK_END);
        long fileSize = ftell(file);
        fseek(file, 0, SEEK_SET);
        char *fileData = (char *)malloc(fileSize + 1);
        if (!fileData)
        {
            printf("Memory allocation failed for file data.\n");
            fclose(file);
            return -1;
        }
        fread(fileData, 1, fileSize, file);
        fileData[fileSize] = '\0';
        fclose(file);
        assembly_data = fileData;
        assembly_data_allocated = true;
    }

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
        if (assembly_data_allocated)
            free((void *)assembly_data);
        return -1;
    }

    /* Compile a simple module */
    HyModuleSourceInfo sourceInfo;
    sourceInfo.sType = HY_STRUCTURE_TYPE_MODULE_SOURCE_INFO;
    sourceInfo.sourceType = HY_MODULE_SOURCE_TYPE_ASSEMBLY;
    sourceInfo.filename = "sample.c";
    sourceInfo.data = (const uint8_t *)assembly_data;

    const HyModuleSourceInfo *sources[] = {&sourceInfo};
    HyModuleCompileInfo compileInfo;
    compileInfo.sType = HY_STRUCTURE_TYPE_MODULE_COMPILE_INFO;
    compileInfo.ppSources = sources;
    compileInfo.sourcesCount = sizeof(sources) / sizeof(sources[0]);

    uint8_t *compiledData = NULL;
    uint32_t compiledDataLen = 0;
    result = hyCompileModule(instance, &compileInfo, &compiledData, &compiledDataLen);
    if (assembly_data_allocated)
        free((void *)assembly_data);
    if (result != HY_RESULT_SUCCESS)
    {
        printf("Module compilation failed. Error code: %d\n", result);
        hyDestroyInstance(instance);
        return -1;
    }

    printf("Module compiled successfully. Compiled data length: %u bytes\n", compiledDataLen);
    putchar('\n');
    printf("Compiled Module Data (Hex):\n");
    print_hex_ascii(compiledData, compiledDataLen, true);
    putchar('\n');

    /* Load the compiled module */
    HyModule *module;
    result = hyLoadModule(instance, compiledData, compiledDataLen, &module);
    if (result != HY_RESULT_SUCCESS)
    {
        printf("Module loading failed. Error code: %d\n", result);
        free(compiledData);
        hyDestroyInstance(instance);
        return -1;
    }

    /* free compiled data if necessary */
    free(compiledData);

    /* Destroy the loaded module */
    hyDestroyModule(module);

    /* Clean up and exit */
    hyDestroyInstance(instance);
    printf("Hycore instance destroyed.\n");

    return 0;
}

void print_hex_ascii(const uint8_t *data, uint32_t length, bool compute_stats)
{
    uint32_t frequency[256] = {0};

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

                // Update frequency count
                frequency[(uint8_t)c]++;
            }
        }
        printf("\n");

        offset += 16;
    }

    if (compute_stats)
    {
        // Compute shanon entropy of the data
        double entropy = 0.0;
        for (int i = 0; i < 256; i++)
        {
            if (frequency[i] > 0)
            {
                double p = (double)frequency[i] / length;
                entropy -= p * log2(p);
            }
        }

        // Display histogram
        printf("Shannon Entropy: %.4f bits/byte (max 8.0000 bits/byte)\n", entropy);
        printf("Number of bytes: %u\n", length);
    }
}

void callback_function(struct HyLogMessageEXT *message)
{
    printf("%s%s[%s:%u] -- %s\n" COLOR_RESET,
           log_level_to_color(message->level),
           log_level_to_string(message->level),
           message->file,
           message->line,
           message->message);
}