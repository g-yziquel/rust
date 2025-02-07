#!/bin/bash

set -eu
set -o pipefail

FAILS_DIR=failures

rm -rf $FAILS_DIR
mkdir $FAILS_DIR

PLATFORM_SUPPORT_FILE=$(rustc +miri --print sysroot)/share/doc/rust/html/rustc/platform-support.html

for target in $(python3 ci/scrape-targets.py $PLATFORM_SUPPORT_FILE); do
    # Wipe the cache before every build to minimize disk usage
    cargo +miri miri clean
    if cargo +miri miri setup --target $target 2>&1 | tee failures/$target; then
        # If the build succeeds, delete its output. If we have output, a build failed.
        rm $FAILS_DIR/$target
    fi
done

# If the sysroot for any target fails to build, we will have a file in FAILS_DIR.
if [[ $(ls failures | wc -l) -ne 0 ]]; then
    echo "Sysroots for the following targets failed to build:"
    ls $FAILS_DIR
    exit 1
fi
