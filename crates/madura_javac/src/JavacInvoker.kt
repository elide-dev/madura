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
    var candidate: java.nio.file.Path? = null
    if (cmd != null) {
      var bin = Paths.get(cmd)
      if (Files.isSymbolicLink(bin)) bin = bin.toRealPath()
      candidate = bin.parent?.parent
    }
    val root =
      if (candidate != null && Files.isRegularFile(candidate.resolve("lib").resolve("modules"))) candidate else null

    // The ambient property is only worth deferring to when it names a real JDK.
    // In the image it is whatever native-image baked in from the build machine,
    // which on most hosts is a path that does not exist — and on the builder
    // itself is a JDK that has nothing to do with this distribution.
    val ambient = System.getProperty("java.home")?.let { Paths.get(it) }
    val ambientUsable = ambient != null && Files.isRegularFile(ambient.resolve("lib").resolve("modules"))

    if (root != null) {
      System.setProperty("java.home", root.toString())
    } else if (!ambientUsable) {
      // Report every input: the interesting failures are the ones where this
      // resolved on the machine that built the binary and not on the one
      // running it, and a bare NoSuchFileException from inside javac names the
      // build machine's path without explaining how it was reached.
      System.err.println("madura: cannot locate the shipped platform image at <root>/lib/modules")
      System.err.println("  argv0 (ProcessHandle): ${cmd ?: "<unavailable>"}")
      System.err.println("  derived root:          ${candidate ?: "<none>"}")
      System.err.println("  ambient java.home:     ${ambient ?: "<unset>"}")
      System.err.println("  java.vm.name:          ${System.getProperty("java.vm.name") ?: "<unset>"}")
      System.exit(2)
      return
    }
    System.exit(Main.compile(args))
  }
}
