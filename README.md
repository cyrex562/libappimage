# libappimage ![CI](https://github.com/AppImage/libappimage/workflows/CI/badge.svg) [![irc](https://img.shields.io/badge/IRC-%23AppImage%20on%20libera.chat-blue.svg)](https://web.libera.chat/#AppImage)

This library is part of the [AppImage](https://github.com/AppImage/appimagekit/) project. It implements functionality for dealing with AppImage files. It is written in C++ and is using Boost.

## Availablility

libappimage is available in the following distributions:
https://repology.org/project/libappimage/versions

## Usage

As a user, you normally do not need to deal with this library. Tools that use it (like [the optional `appimaged` daemon](https://github.com/AppImage/appimaged)) usually come with a bundled copy of it.

## API documentation

As a developer interested in using libappimage in your projects, please find the API documentation here:
https://docs.appimage.org/api/libappimage/. Please note that if you are using libappimage in your project, we recommend bundling your private copy or linking statically to it, since the versions provided by distributions may be outdated.

## Building

To build the static library:

```bash
cargo build --release
```

This will generate a static library file in `target/release/`:
- On Linux: `libappimage.a`
- On macOS: `libappimage.a`
- On Windows: `appimage.lib`

## Usage in C/C++

To use this library in a C/C++ project:

1. Include the header file (to be generated)
2. Link against the static library
3. Call the exported functions

Example:

```c
#include <appimage.h>

int main() {
    appimage_init();
    const char* version = appimage_version();
    printf("AppImage version: %s\n", version);
    return 0;
}
```

## Development

To run tests:

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Your contributions are welcome.

If you make or suggest changes to this code, please test that the resulting executables (like [the `appimaged` daemon](https://github.com/AppImage/appimaged)) are still working properly.


If you have questions, AppImage developers are on #AppImage on irc.libera.chat.
