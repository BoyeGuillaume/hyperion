#ifndef _HYCORE_EXCEPT_HPP
#define _HYCORE_EXCEPT_HPP

#include <hycore/generated_version.h>

#ifdef HY_BUILD_SHARED
#if defined(_WIN32) || defined(__CYGWIN__)
#ifdef HYCORE_EXPORTS
#define HYCORE_API __declspec(dllexport)
#define HYCORE_PRIVATE_API
#else
#define HYCORE_API __declspec(dllimport)
#define HYCORE_PRIVATE_API
#endif
#elif defined(__GNUC__) && __GNUC__ >= 4
#ifdef HYCORE_EXPORTS
#define HYCORE_API __attribute__((visibility("default")))
#define HYCORE_PRIVATE_API __attribute__((visibility("hidden")))
#else
#define HYCORE_API
#define HYCORE_PRIVATE_API unreachable
#endif
#else
#error "Unknown shared library mechanism for this platform"
#endif
#else
#define HYCORE_API
#define HYCORE_PRIVATE_API
#endif

#endif // _HYCORE_EXCEPT_HPP
