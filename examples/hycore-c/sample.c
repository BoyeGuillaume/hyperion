#include <hycore.h>
#include <math.h>
#include <stdio.h>

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

static const char *log_level_to_color(HyLoggerLevel level) {
  switch (level) {
  case HY_LOGGER_LEVEL_TRACE:
    return COLOR_BRIGHT_BLACK;
  case HY_LOGGER_LEVEL_DEBUG:
    return COLOR_BLUE;
  case HY_LOGGER_LEVEL_INFO:
    return COLOR_GREEN;
  case HY_LOGGER_LEVEL_WARN:
    return COLOR_YELLOW;
  case HY_LOGGER_LEVEL_ERROR:
    return COLOR_RED;
  default:
    return COLOR_RESET;
  }
}

static const char *log_level_to_string(HyLoggerLevel level) {
  switch (level) {
  case HY_LOGGER_LEVEL_TRACE:
    return "[TRACE]";
  case HY_LOGGER_LEVEL_DEBUG:
    return "[DEBUG ]";
  case HY_LOGGER_LEVEL_INFO:
    return "[INFO  ]";
  case HY_LOGGER_LEVEL_WARN:
    return "[WARN  ]";
  case HY_LOGGER_LEVEL_ERROR:
    return "[ERROR]";
  default:
    return "[UNKNOWN]";
  }
}

void callback_function(const HyLoggerRecord *pMessage, void *pUserData);
void print_hex_ascii(const uint8_t *data, uint32_t length, bool compute_stats);
void print_error_message();

int main(int argc, char **argv) {
  // if (argc < 1)
  //   return 1;
  // if (argc != 2) {
  //   printf("Usage: %s <assembly_file>\n", argv[0]);
  //   return -1;
  // }

  /* Retrieve and print Hycore version information */
  HyVersionInfo version;
  hyGetVersionInfo(&version);
  printf("Hycore Version: %u.%u.%u\n", version.major, version.minor,
         version.patch);

  /* Construct a new instance */
  HyApplicationInfo appInfo;
  appInfo.sType = HY_STRUCTURE_TYPE_APPLICATION_INFO;
  appInfo.applicationVersion = version;
  appInfo.pApplicationName = "SimpleCApp";
  appInfo.engineVersion = version;
  appInfo.pEngineName = "HycoreEngine";

  HyLoggerPluginCreateInfo loggerPluginCreateInfo;
  loggerPluginCreateInfo.sType = HY_STRUCTURE_TYPE_LOGGER_PLUGIN_CREATE_INFO;
  loggerPluginCreateInfo.level = HY_LOGGER_LEVEL_DEBUG;
  loggerPluginCreateInfo.pSinkCallback = callback_function;
  loggerPluginCreateInfo.pUserData = NULL;
  loggerPluginCreateInfo.pNext = NULL;

  const char *extensions[] = {
      HY_LOGGER_PLUGIN_NAME,
  };
  HyInstanceCreateInfo createInfo;
  createInfo.sType = HY_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
  createInfo.pApplicationInfo = &appInfo;
  createInfo.ppEnabledPlugins = extensions;
  createInfo.enabledPluginCount = sizeof(extensions) / sizeof(extensions[0]);
  createInfo.nodeRank = 0;
  createInfo.pNext = &loggerPluginCreateInfo;

  HyInstance *instance;
  if (hyCreateInstance(&createInfo, &instance) < 0) {
    print_error_message();
    return -1;
  }

  printf("Instance created successfully.\n");

  hyDestroyInstance(instance);

  printf("Instance destroyed successfully.\n");
  return 0;
}

void print_hex_ascii(const uint8_t *data, uint32_t length, bool compute_stats) {
  uint32_t frequency[256] = {0};

  uint32_t offset = 0;
  while (offset < length) {
    printf("%08X | ", offset); /* offset */
    for (uint32_t i = 0; i < 16; i++) {
      if (offset + i < length)
        printf("%02X ", data[offset + i]);
      else
        printf("   ");
    }
    printf("| ");
    for (uint32_t i = 0; i < 16; i++) {
      if (offset + i < length) {
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

  if (compute_stats) {
    // Compute shanon entropy of the data
    double entropy = 0.0;
    for (int i = 0; i < 256; i++) {
      if (frequency[i] > 0) {
        double p = (double)frequency[i] / length;
        entropy -= p * log2(p);
      }
    }

    // Display histogram
    printf("Shannon Entropy: %.4f bits/byte (max 8.0000 bits/byte)\n", entropy);
    printf("Number of bytes: %u\n", length);
  }
}

void print_error_message() {
  char errorBuffer[256];
  char backtraceBuffer[1024];
  if (hyGetLastError(errorBuffer, sizeof(errorBuffer), backtraceBuffer,
                     sizeof(backtraceBuffer))) {
    printf("Failed to retrieve error message.\n");
    return;
  }
  printf("Error: %s\nBacktrace:\n%s\n", errorBuffer, backtraceBuffer);
}

void callback_function(const HyLoggerRecord *pRecord, void *pUserData) {
  if (pUserData != NULL) {
    printf("User data: %p\n", pUserData);
  }

  printf("%s%s[%s:%u] -- %s\n" COLOR_RESET, log_level_to_color(pRecord->level),
         log_level_to_string(pRecord->level), pRecord->pFile, pRecord->line,
         pRecord->pMessage);
}
