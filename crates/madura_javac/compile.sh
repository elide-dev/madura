#!/bin/bash

mkdir -p .dev/artifacts/native-image .dev/artifacts/jvm
elide kotlinc -- -d .dev/artifacts/jvm/madura_javac.jar ./src/JavacInvoker.kt

elide native-image -- \
    --shared \
    --verbose \
    -H:+UnlockExperimentalVMOptions \
    -H:-CheckToolchain \
    -H:+AllowJRTFileSystem \
    -H:IncludeResourceBundles=com.sun.tools.javac.resources.compiler \
    -H:IncludeResourceBundles=com.sun.tools.javac.resources.javac \
    -H:IncludeResourceBundles=com.sun.tools.javac.resources.version \
    -H:-UnlockExperimentalVMOptions \
    -o .dev/artifacts/native-image/libmadura.so \
    -cp .dev/artifacts/jvm/madura_javac.jar \
    dev.elide.jvm.JavacInvoker

echo "Done."
