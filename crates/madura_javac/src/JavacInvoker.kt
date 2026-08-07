package dev.elide.jvm

import com.sun.tools.javac.Main
import java.nio.file.Files
import java.nio.file.Paths

object JavacInvoker {
  // In the native image `java.home` is unset: resolve the dist root from the
  // binary's own absolute path (<root>/bin/madura, or target/<profile>/madura
  // in the dev tree), mirroring Elide Entry.kt's binpath resolution. On a
  // plain JVM java.home is already valid and this block is skipped.
  @JvmStatic fun main(args: Array<String>) {
    if (System.getProperty("java.home") == null) {
      val cmd = ProcessHandle.current().info().command().orElse(null)
      if (cmd == null) {
        System.err.println("madura: cannot resolve own binary path")
        System.exit(2)
        return
      }
      var bin = Paths.get(cmd)
      if (Files.isSymbolicLink(bin)) bin = bin.toRealPath()
      val root = bin.parent?.parent
      if (root == null || !Files.isRegularFile(root.resolve("lib").resolve("modules"))) {
        System.err.println(
          "madura: missing platform image at <root>/lib/modules (binary must live in <root>/bin or target/<profile>)")
        System.exit(2)
        return
      }
      System.setProperty("java.home", root.toString())
    }
    System.exit(Main.compile(args))
  }
}
