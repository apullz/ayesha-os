[app]

# (str) Title of your application
title = Ayesha Core

# (str) Package name
package.name = ayesha_core

# (str) Package domain
package.domain = org.ayesha

# (source.dir) Source code directory where the main.py live
source.dir = .

# (list) Source files to include
source.include_exts = py,png,jpg,kv,atlas

# (list) List of inclusions using pattern matching
# include_exts = png,jpg

# (list) List of exclusions using pattern matching
exclude_exts = spec

# (list) List of directory to exclude (let empty to not exclude anything)
exclude_dirs = tests, bin, .github

# (str) Application versioning
version = 1.0.0

# (str) Application requirements
# comma seperated e.g. requirements = sqlite3,kivy
requirements = python3,kivy

# (str) Supported orientation (landscape, sensorLandscape, portrait or sensorPortrait)
orientation = portrait

# (bool) Indicate if the application should be fullscreen or not
fullscreen = 0

# (string) Presplash of the application (image or drawable resource)
# presplash.filename = %(source.dir)s/data/presplash.png

# (str) Icon of the application
# icon.filename = %(source.dir)s/data/icon.png

# (str) Supported orientations
# valid values: landscape, portrait, portrait-reverse, landscape-reverse

# (list) Permissions
android.permissions = INTERNET

# (int) Target Android API
android.api = 31

# (int) Minimum Android API
android.minapi = 21

# (str) Android NDK version to use
android.ndk = 25b

# (bool) Use the new toolchain
android.gradle_dependencies = 

# (bool) Enable AndroidX support
android.enable_androidx = True

# (bool) Copy library instead of making a libpymodules.so
android.copy_libs = 1

# (str) Android logcat filters to use
#android.logcat_filters = *:S python:D

# (bool) Copy library instead of making a libpymodules.so
# android.add_src = 

# (bool) Indicate if the application should be fullscreen or not
# android.fullscreen = 0

# (str) Android app theme, default is ok for Kivy-based app
# android.theme = "@android:style/Theme.NoTitleBar"

# (bool) Copy library instead of making a libpymodules.so
android.accept_sdk_license = True

[buildozer]

# (int) Log level (0 = error only, 1 = info, 2 = debug (with command output))
log_level = 2

# (int) Display warning as error
warn_on_root = 1

# (str) Path to build artifact storage, absolute or relative to spec file
build_dir = .buildozer

# (str) Path to build output (i.e. .apk, .ipa) storage
bin_dir = ./bin
