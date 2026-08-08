package dev.elide.jvm

import com.sun.tools.javac.Main
import java.nio.file.Files
import java.nio.file.Paths

object JavacInvoker {
  // Resolve the dist root from the binary's own absolute path (<root>/bin/madura,
  // or target/<profile>/madura in the dev tree), mirroring Elide Entry.kt's
  // binpath resolution, and always prefer the platform image found there.
  //
  // Testing `java.home == null` is not sufficient, even though it is unset in
  // most native images: native-image can bake the *build* machine's java.home
  // into the image, and on any host where that path happens to exist javac
  // silently reads a JDK belonging to whoever built the binary rather than the
  // one shipped beside it. That is how CI compiled against the runner's GraalVM
  // and then failed on a machine without it.
  //
  // The ambient property is honoured only when the binary is not sitting in a
  // distribution layout at all — running the app jar on a plain JVM, where
  // java.home is already correct and there is no <root>/lib/modules to prefer.
  @JvmStatic fun main(args: Array<String>) {
    val cmd = ProcessHandle.current().info().command().orElse(null)
    var root: java.nio.file.Path? = null
    if (cmd != null) {
      var bin = Paths.get(cmd)
      if (Files.isSymbolicLink(bin)) bin = bin.toRealPath()
      val candidate = bin.parent?.parent
      if (candidate != null && Files.isRegularFile(candidate.resolve("lib").resolve("modules"))) {
        root = candidate
      }
    }
    if (root != null) {
      System.setProperty("java.home", root.toString())
    } else if (System.getProperty("java.home") == null) {
      System.err.println(
        "madura: missing platform image at <root>/lib/modules (binary must live in <root>/bin or target/<profile>)")
      System.exit(2)
      return
    }
    System.exit(Main.compile(args))
  }
}
