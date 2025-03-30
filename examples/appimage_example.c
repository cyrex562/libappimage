#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "appimage.h"

void log_callback(int level, const char* message) {
    const char* level_str;
    switch (level) {
        case APPIMAGE_LOG_DEBUG:
            level_str = "DEBUG";
            break;
        case APPIMAGE_LOG_INFO:
            level_str = "INFO";
            break;
        case APPIMAGE_LOG_WARNING:
            level_str = "WARNING";
            break;
        case APPIMAGE_LOG_ERROR:
            level_str = "ERROR";
            break;
        default:
            level_str = "UNKNOWN";
            break;
    }
    printf("[%s] %s\n", level_str, message);
}

void print_error(void) {
    const char* error = appimage_get_last_error();
    if (error) {
        fprintf(stderr, "Error: %s\n", error);
    }
}

int main(int argc, char* argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s <appimage_path>\n", argv[0]);
        return 1;
    }

    // Set up logging
    appimage_set_log_level(APPIMAGE_LOG_INFO);
    appimage_set_log_callback(log_callback);

    // Create AppImage instance
    appimage_t appimage = appimage_new(argv[1]);
    if (!appimage) {
        print_error();
        return 1;
    }

    // Get AppImage format
    int format = appimage_get_format(appimage);
    if (format < 0) {
        print_error();
        appimage_free(appimage);
        return 1;
    }
    printf("AppImage format: Type %d\n", format);

    // Get AppImage size
    uint64_t size = appimage_get_size(appimage);
    if (size == 0) {
        print_error();
        appimage_free(appimage);
        return 1;
    }
    printf("AppImage size: %lu bytes\n", (unsigned long)size);

    // Calculate MD5 hash
    char hash[33];
    if (appimage_get_md5(appimage, hash, sizeof(hash)) != APPIMAGE_SUCCESS) {
        print_error();
        appimage_free(appimage);
        return 1;
    }
    printf("AppImage MD5: %s\n", hash);

    // Extract .DirIcon
    const char* icon_path = "icon.png";
    if (appimage_extract_file(appimage, ".DirIcon", icon_path) != APPIMAGE_SUCCESS) {
        print_error();
        appimage_free(appimage);
        return 1;
    }
    printf("Extracted .DirIcon to %s\n", icon_path);

    // Check integration status
    int integrated = appimage_is_integrated(appimage);
    if (integrated < 0) {
        print_error();
        appimage_free(appimage);
        return 1;
    }
    printf("AppImage is %sintegrated\n", integrated ? "" : "not ");

    // Integrate AppImage if not already integrated
    if (!integrated) {
        printf("Integrating AppImage...\n");
        if (appimage_integrate(appimage) != APPIMAGE_SUCCESS) {
            print_error();
            appimage_free(appimage);
            return 1;
        }
        printf("AppImage integrated successfully\n");
    }

    // Unintegrate AppImage
    printf("Unintegrating AppImage...\n");
    if (appimage_unintegrate(appimage) != APPIMAGE_SUCCESS) {
        print_error();
        appimage_free(appimage);
        return 1;
    }
    printf("AppImage unintegrated successfully\n");

    // Clean up
    appimage_free(appimage);
    printf("Done\n");

    return 0;
} 