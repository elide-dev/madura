package dev.elide.jvm

import com.sun.source.util.JavacTask
import com.sun.tools.javac.Main
import java.nio.file.Files
import java.nio.file.Paths
import javax.tools.Diagnostic
import javax.tools.DiagnosticListener
import javax.tools.JavaFileObject
import javax.tools.ToolProvider

object JavacInvoker {
  private fun runMainCompile(checkOnly: Boolean, args: Array<String>): Int {
    return if (checkOnly) runCheck(args) else Main.compile(args)
  }

  // Check mode drives javac through the public JavacTask API instead of the
  // CLI: `analyze()` runs parse, enter, attribution and flow — annotation
  // processing included, so every error and warning surfaces — and the task
  // is dropped before `generate()`, so nothing is ever written to disk. Exit
  // codes follow javac's convention: 1 when errors were reported, else 0.
  private fun runCheck(args: Array<String>): Int {
    val compiler =
      ToolProvider.getSystemJavaCompiler() ?: error("no system Java compiler in this image")
    var errors = 0
    val listener = DiagnosticListener<JavaFileObject> { diag ->
      if (diag.kind == Diagnostic.Kind.ERROR) errors++
      System.err.println(diag)
    }
    compiler.getStandardFileManager(listener, null, null).use { fm ->
      // Sources are recognized by suffix; everything else on the command line
      // is an option (values like `-d <dir>` ride along in order).
      val (sources, options) = args.toList().partition { it.endsWith(".java") }
      val task =
        compiler.getTask(null, fm, listener, options, null, fm.getJavaFileObjectsFromStrings(sources))
      (task as JavacTask).analyze()
    }
    return if (errors > 0) 1 else 0
  }

  fun compileMain(args: Array<String>): Int {
    return Main.compile(args)
  }

  // Point `java.home` at the caller's JDK so the embedded javac reads its
  // platform metadata — `lib/modules` (the JRT filesystem) and `lib/ct.sym`
  // (`--release` targeting) — from `$JAVA_HOME`. Returns a diagnostic message
  // when JAVA_HOME is unset or does not name a jimage-bearing JDK, else null.
  private fun applyJavaHome(): String? {
    val javaHome = System.getenv("JAVA_HOME")
    if (javaHome.isNullOrEmpty())
      return "JAVA_HOME is not set; point it at a JDK (source of lib/modules and lib/ct.sym)"
    if (!Files.isRegularFile(Paths.get(javaHome, "lib", "modules")))
      return "JAVA_HOME does not name a jimage-bearing JDK (no lib/modules): $javaHome"
    System.setProperty("java.home", javaHome)
    return null
  }

  // The native-image entrypoint and the whole CLI. The first argument selects
  // the mode: `check` runs javac without codegen, `compile` names the default
  // explicitly, and anything else is javac's own first argument, so the binary
  // stands in for `javac` (drop-in form). Platform metadata is resolved from
  // `$JAVA_HOME`; a missing or invalid one is an environment error (exit 2).
  @JvmStatic fun main(args: Array<String>) {
    var checkOnly = false
    var rest = args
    if (args.isNotEmpty()) when (args[0]) {
      "check" -> {
        checkOnly = true
        rest = args.copyOfRange(1, args.size)
      }
      "compile" -> rest = args.copyOfRange(1, args.size)
    }

    val homeError = applyJavaHome()
    val code =
      if (homeError != null) {
        System.err.println("madura: $homeError")
        2
      } else {
        runMainCompile(checkOnly, rest)
      }
    System.exit(code)
  }
}
