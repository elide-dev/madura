package dev.elide.jvm

import com.sun.source.util.JavacTask
import com.sun.tools.javac.Main
import org.graalvm.nativeimage.ImageInfo
import org.graalvm.nativeimage.ProcessProperties
import java.nio.file.Files
import java.nio.file.Paths
import javax.tools.Diagnostic
import javax.tools.DiagnosticListener
import javax.tools.JavaFileObject
import javax.tools.ToolProvider

object JavacInvoker {
  // Intercepted before javac sees it: forces the platform-metadata root.
  private const val JAVA_HOME_FLAG = "--java-home"

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

  // A directory is a valid platform-metadata root when it holds `lib/modules`;
  // `lib/ct.sym` sits beside it for `--release` targeting.
  private fun hasModules(dir: String): Boolean {
    return Files.isRegularFile(Paths.get(dir, "lib", "modules"))
  }

  // Resolve the platform-metadata root — a directory holding `lib/modules` and
  // `lib/ct.sym` — in order of precedence:
  //   1. an explicit `--java-home <dir>` override,
  //   2. binary-relative `<exe>/../<os.arch>`, the shipped hermetic layout,
  //   3. the `$JAVA_HOME` environment variable.
  // Returns null when nothing resolves to a jimage-bearing directory.
  private fun resolvePlatformHome(forced: String?): String? {
    if (forced != null) return if (hasModules(forced)) forced else null

    // Binary-relative: `<exe>/../lib/modules`. Only meaningful in the native
    // image, where the executable path is knowable; a symlinked launcher is
    // resolved to its real location first, matching the shipped layout.
    if (ImageInfo.inImageCode()) {
      try {
        val exe = ProcessProperties.getExecutableName()
        if (exe != null) {
          val here = Paths.get(exe).toRealPath().parent
          if (here != null && hasModules(here.toString())) return here.toString()
        }
      } catch (_: Exception) {
        // Unresolvable executable path: fall through to $JAVA_HOME.
      }
    }

    val env = System.getenv("JAVA_HOME")
    if (!env.isNullOrEmpty() && hasModules(env)) return env
    return null
  }

  private fun platformError(forced: String?): String {
    return if (forced != null)
      "$JAVA_HOME_FLAG does not name a jimage-bearing JDK (no lib/modules): $forced"
    else
      "cannot locate platform metadata (lib/modules); set JAVA_HOME or pass $JAVA_HOME_FLAG <dir>"
  }

  // The native-image entrypoint and the whole CLI. The first argument selects
  // the mode: `check` runs javac without codegen, `compile` names the default
  // explicitly, and anything else is javac's own first argument, so the binary
  // stands in for `javac` (drop-in form). A `--java-home <dir>` anywhere in the
  // arguments forces the platform-metadata root and is never forwarded to
  // javac; otherwise the root is resolved binary-relative, then from $JAVA_HOME.
  @JvmStatic fun main(args: Array<String>) {
    var checkOnly = false
    var start = 0
    if (args.isNotEmpty()) when (args[0]) {
      "check" -> {
        checkOnly = true
        start = 1
      }
      "compile" -> start = 1
    }

    var forced: String? = null
    var missingValue = false
    val forward = ArrayList<String>(args.size)
    var i = start
    while (i < args.size) {
      val arg = args[i]
      if (arg == JAVA_HOME_FLAG) {
        if (i + 1 < args.size) {
          forced = args[i + 1]
          i += 2
        } else {
          missingValue = true
          i += 1
        }
      } else if (arg.startsWith("$JAVA_HOME_FLAG=")) {
        forced = arg.substring(JAVA_HOME_FLAG.length + 1)
        i += 1
      } else {
        forward.add(arg)
        i += 1
      }
    }

    val code: Int
    if (missingValue) {
      System.err.println("madura: $JAVA_HOME_FLAG requires a directory")
      code = 2
    } else {
      val home = resolvePlatformHome(forced)
      if (home == null) {
        System.err.println("madura: ${platformError(forced)}")
        code = 2
      } else {
        System.setProperty("java.home", home)
        code = runMainCompile(checkOnly, forward.toTypedArray())
      }
    }
    System.exit(code)
  }
}
