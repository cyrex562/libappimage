#ifndef APPIMAGE_H
#define APPIMAGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Error codes returned by the AppImage API functions.
 */
typedef enum {
    APPIMAGE_SUCCESS = 0,           /**< Operation succeeded */
    APPIMAGE_IO_ERROR = 1,          /**< IO error occurred */
    APPIMAGE_INVALID_FORMAT = 2,     /**< Invalid format */
    APPIMAGE_ELF_ERROR = 3,         /**< ELF error */
    APPIMAGE_FILESYSTEM_ERROR = 4,   /**< File system error */
    APPIMAGE_ARCHIVE_ERROR = 5,      /**< Archive error */
    APPIMAGE_SQUASHFS_ERROR = 6,     /**< SquashFS error */
    APPIMAGE_NOT_SUPPORTED = 7,      /**< Operation not supported */
    APPIMAGE_INVALID_PARAMETER = 8,  /**< Invalid parameter */
    APPIMAGE_NOT_FOUND = 9,         /**< Resource not found */
    APPIMAGE_PERMISSION_DENIED = 10, /**< Permission denied */
    APPIMAGE_OPERATION_FAILED = 11,  /**< Operation failed */
    APPIMAGE_STRING_ERROR = 12,      /**< String conversion error */
} appimage_error_t;

/**
 * Log levels for the AppImage API.
 */
typedef enum {
    APPIMAGE_LOG_DEBUG = 0,    /**< Debug level */
    APPIMAGE_LOG_INFO = 1,     /**< Info level */
    APPIMAGE_LOG_WARNING = 2,  /**< Warning level */
    APPIMAGE_LOG_ERROR = 3,    /**< Error level */
} appimage_log_level_t;

/**
 * Opaque handle to an AppImage instance.
 */
typedef void* appimage_t;

/**
 * Set the log level for the AppImage API.
 * @param level The log level to set
 * @return Error code
 */
int appimage_set_log_level(int level);

/**
 * Set the log callback function.
 * @param callback The callback function to set
 * @return Error code
 */
int appimage_set_log_callback(void (*callback)(int level, const char* message));

/**
 * Get the last error message.
 * @return The last error message or NULL if no error occurred
 */
const char* appimage_get_last_error(void);

/**
 * Create a new AppImage instance.
 * @param path Path to the AppImage file
 * @return AppImage handle or NULL on error
 */
appimage_t appimage_new(const char* path);

/**
 * Free an AppImage instance.
 * @param appimage AppImage handle
 */
void appimage_free(appimage_t appimage);

/**
 * Get the AppImage format.
 * @param appimage AppImage handle
 * @return Format type (1 for Type 1, 2 for Type 2, 0 for unknown) or error code
 */
int appimage_get_format(const appimage_t appimage);

/**
 * Extract a file from the AppImage.
 * @param appimage AppImage handle
 * @param source Source path within the AppImage
 * @param target Target path to extract to
 * @return Error code
 */
int appimage_extract_file(const appimage_t appimage, const char* source, const char* target);

/**
 * Get the AppImage size.
 * @param appimage AppImage handle
 * @return Size in bytes or 0 on error
 */
uint64_t appimage_get_size(const appimage_t appimage);

/**
 * Calculate the AppImage MD5 hash.
 * @param appimage AppImage handle
 * @param hash Buffer to store the hash
 * @param hash_len Length of the hash buffer
 * @return Error code
 */
int appimage_get_md5(const appimage_t appimage, char* hash, int hash_len);

/**
 * Integrate the AppImage into the system.
 * @param appimage AppImage handle
 * @return Error code
 */
int appimage_integrate(const appimage_t appimage);

/**
 * Remove the AppImage integration from the system.
 * @param appimage AppImage handle
 * @return Error code
 */
int appimage_unintegrate(const appimage_t appimage);

/**
 * Check if the AppImage is integrated.
 * @param appimage AppImage handle
 * @return 1 if integrated, 0 if not integrated, or error code
 */
int appimage_is_integrated(const appimage_t appimage);

#ifdef __cplusplus
}
#endif

#endif /* APPIMAGE_H */ 