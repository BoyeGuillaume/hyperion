#include <hycore.h>
#include <math.h>
#include <memory.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <threads.h>
#include <unistd.h>

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

static volatile bool gShouldExit = false;

static const char *logLevelToColour(HyLoggerLevel level);
static const char *logLevelToString(HyLoggerLevel level);
static void logCallback(const HyLoggerRecord *pMessage, void *pUserData);
static void printHexASCII(const uint8_t *data, uint32_t length,
                          bool compute_stats);
static void printErrorMessage();
static void handleInterruptSignal(int signal);

int main(int argc, char **argv) {
  if (argc < 1)
    return 1;
  if (argc != 2) {
    printf("Usage: %s <assembly_file>\n", argv[0]);
    return -1;
  }

  /* Retrieve and print Hycore version information */
  HyVersionInfo version;
  hyGetVersionInfo(&version);
  printf("Hycore Version: %u.%u.%u\n", version.major, version.minor,
         version.patch);

  /* Construct a new instance */
  HyApplicationInfo appInfo;
  appInfo.sType = HY_STRUCTURE_TYPE_APPLICATION_INFO;
  appInfo.applicationVersion.major = 1;
  appInfo.applicationVersion.minor = 0;
  appInfo.applicationVersion.patch = 0;
  appInfo.pApplicationName = "SimpleCApp";
  appInfo.engineVersion = version;
  appInfo.pEngineName = "HycoreEngine";

  HyLoggerPluginCreateInfo loggerPluginCreateInfo;
  loggerPluginCreateInfo.sType = HY_STRUCTURE_TYPE_LOGGER_PLUGIN_CREATE_INFO;
  loggerPluginCreateInfo.level = HY_LOGGER_LEVEL_TRACE;
  loggerPluginCreateInfo.pSinkCallback = logCallback;
  loggerPluginCreateInfo.pUserData = NULL;
  loggerPluginCreateInfo.pNext = NULL;

  const char *extensions[] = {
      HY_LOGGER_PLUGIN_NAME,
      HY_REMOTE_PLUGIN_NAME,
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
    printErrorMessage();
    return -1;
  }

  // Compile the assembly file
  HyModuleCompileInfoSourceDescriptor sourceDescriptors[1];
  sourceDescriptors[0].pFilename = argv[1];
  sourceDescriptors[0].filenameSize = (uint32_t)strlen(argv[1]);
  sourceDescriptors[0].pData = NULL;

  HyModuleCompileInfo compileInfo;
  compileInfo.sType = HY_STRUCTURE_TYPE_MODULE_COMPILE_INFO;
  compileInfo.pBasePath = NULL;
  compileInfo.pSourceDescriptors = sourceDescriptors;
  compileInfo.sourceDescriptorCount =
      sizeof(sourceDescriptors) / sizeof(sourceDescriptors[0]);
  compileInfo.flags = HY_MODULE_COMPILE_INFO_FLAG_BITS_ENABLE_ZSTD_COMPRESSION;
  compileInfo.pNext = NULL;

  uint8_t *outputBuffer;
  uint32_t outputBufferSize;
  if (hyCompileModule(instance, &compileInfo, &outputBuffer,
                      &outputBufferSize) < 0) {
    // printErrorMessage();
    hyDestroyInstance(instance);
    return -1;
  }

  printf("Compiled module size: %u bytes\n", outputBufferSize);
  printHexASCII(outputBuffer, outputBufferSize, true);

  /* Load the compiled module into the instance */
  HyModule module;
  if (hyLoadCompiledModule(instance, outputBuffer, outputBufferSize, &module) <
      0) {
    // printErrorMessage();
    hyFreeCompiledModuleBuffer(outputBuffer);
    hyDestroyInstance(instance);
    return -1;
  }
  hyFreeCompiledModuleBuffer(outputBuffer);

  /* Launch the remote module on the instance */
  HyStartRemoteServerInfo launchInfo;
  launchInfo.sType = HY_STRUCTURE_TYPE_START_REMOTE_SERVER_INFO;
  launchInfo.maxConnections = 256;
  launchInfo.pHost = "127.0.0.1";
  launchInfo.port = 8080;
  launchInfo.pNext = NULL;

  if (hyStartRemoteServer(instance, &launchInfo) < 0) {
    // printErrorMessage();
    hyDestroyModule(instance, module);
    hyDestroyInstance(instance);
    return -1;
  }

  /* Wait until we get a cancel signal (e.g. Ctrl+C) */
  printf("Module launched. Press Ctrl+C to exit.\n");
  signal(
      SIGINT,
      handleInterruptSignal); // Register signal handler for graceful shutdown
  while (!gShouldExit) {
    usleep(50);
  }
  printf("Ctrl+C received. Shutting down...\n");
  signal(SIGINT, SIG_DFL); // Restore default signal handler

  /* Finally gracefully shutdown the remote server */
  if (hyShutdownRemoteServer(instance) < 0) {
    // printErrorMessage();
    hyDestroyModule(instance, module);
    hyDestroyInstance(instance);
    return -1;
  }

  /* Finally destroy the module and instance */
  hyDestroyModule(instance, module);
  hyDestroyInstance(instance);
  return 0;
}

static void printHexASCII(const uint8_t *data, uint32_t length,
                          bool compute_stats) {
  uint32_t frequency[256] = {0};

  uint32_t offset = 0;
  while (offset < length) {
    printf(COLOR_BRIGHT_BLACK "%08X " COLOR_RESET " | ", offset); /* offset */
    for (uint32_t i = 0; i < 16; i++) {
      if (offset + i < length) {
        uint8_t byte = data[offset + i];
        if (byte == 0) {
          printf(COLOR_BRIGHT_BLACK "%02X " COLOR_RESET, byte);
        } else {
          printf("%02X ", byte);
        }
      } else
        printf("   ");
    }
    printf("| ");
    for (uint32_t i = 0; i < 16; i++) {
      if (offset + i < length) {
        char c = data[offset + i];
        if (c >= 32 && c <= 126)
          printf("%c", c);
        else
          printf(COLOR_BRIGHT_BLACK "." COLOR_RESET);

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
    printf(COLOR_BRIGHT_BLACK "Shannon Entropy:" COLOR_RESET
                              " %.4f " COLOR_BRIGHT_BLACK
                              "bits/byte (max 8.0000 bits/byte)\n" COLOR_RESET,
           entropy);
    printf(COLOR_BRIGHT_BLACK "Number of bytes: " COLOR_RESET "%u\n", length);
  }
}

static void printStringLinePrefix(const char *prefix, const char *str,
                                  bool skipFirstLinePrefix) {
  bool firstLine = skipFirstLinePrefix;

  while (*str) {
    // Find position of first newline in string
    if (!firstLine) {
      printf("%s", prefix);
    }
    firstLine = false;

    while (*str && *str != '\n') {
      putchar(*str++);
    }
    if (*str == '\n') {
      putchar('\n');
      str++;
    }
  }
}

static void printErrorMessage() {
  char errorBuffer[256];
  char backtraceBuffer[1024];
  if (hyGetLastError(errorBuffer, sizeof(errorBuffer), backtraceBuffer,
                     sizeof(backtraceBuffer)) == 0) {
    printf("Failed to retrieve error message.\n");
    return;
  }
  printf(COLOR_RED "Error: %s", errorBuffer);
  printf(COLOR_BRIGHT_BLACK "\n\nBacktrace: ");
  printStringLinePrefix("           ", backtraceBuffer, true);
  printf("\n" COLOR_RESET);
}

static void logCallback(const HyLoggerRecord *pRecord, void *pUserData) {
  if (pUserData != NULL) {
    printf("User data: %p\n", pUserData);
  }

  printf("%s%s[%s:%u] -- %s\n" COLOR_RESET, logLevelToColour(pRecord->level),
         logLevelToString(pRecord->level), pRecord->pFile, pRecord->line,
         pRecord->pMessage);
}

static const char *logLevelToColour(HyLoggerLevel level) {
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

static const char *logLevelToString(HyLoggerLevel level) {
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

static void handleInterruptSignal(int signal) {
  if (signal == SIGINT) {
    gShouldExit = true;
  }
}
