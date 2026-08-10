package dev.elide.jvm

import com.sun.source.util.JavacTask
import com.sun.tools.javac.Main
import org.graalvm.nativeimage.c.function.CEntryPoint
import org.graalvm.nativeimage.IsolateThread
import org.graalvm.nativeimage.c.type.CCharPointer
import org.graalvm.nativeimage.c.type.CCharPointerPointer
import org.graalvm.nativeimage.c.type.CTypeConversion
import java.nio.file.Files
import java.nio.file.Paths
import java.nio.file.Path
import javax.tools.Diagnostic
import javax.tools.DiagnosticListener
import javax.tools.JavaFileObject
import javax.tools.ToolProvider

object JavacInvoker {
  private fun resolveBinPath(cmd: String?): Path? {
    var candidate: java.nio.file.Path? = null
    if (cmd != null) {
      var bin = Paths.get(cmd)
      if (Files.isSymbolicLink(bin)) bin = bin.toRealPath()
      candidate = bin.parent?.parent
    }
    val root =
      if (candidate != null && Files.isRegularFile(candidate.resolve("lib").resolve("modules"))) candidate else null
    return root
  }

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

  // Plain javac passthrough for JVM runs (`elide run`, the app jar) and for
  // elide's required native-image entrypoint. The shipped CLI is the Rust
  // binary, which resolves java.home itself and enters via `compile_javac`.
  @JvmStatic fun main(args: Array<String>) {
    System.exit(compileMain(args))
  }

  // The host resolves the dist root and passes it as homePath; it is trusted
  // as-is, with no lib/modules validation. binPath is diagnostic-only.
  @CEntryPoint(name = "compile_javac") @JvmStatic fun compileJavac(
    isolate: IsolateThread,
    binPath: CCharPointer,
    homePath: CCharPointer,
    argCount: Int,
    argArray: CCharPointerPointer,
    checkOnly: Boolean,
  ): Int {
    try {
      val home = if (homePath.isNonNull) CTypeConversion.toJavaString(homePath) else null
      if (!home.isNullOrEmpty()) System.setProperty("java.home", home)
      val args = ArrayList<String>(argCount + 1)
      // Plain loop: capturing a word-typed value (argArray) in a lambda is
      // rejected by native-image ("Expected Object but got Word").
      for (i in 0 until argCount) args.add(CTypeConversion.toJavaString(argArray.read(i)))
      return runMainCompile(checkOnly, args.toTypedArray())
    } catch (err: Throwable) {
      val bin = if (binPath.isNonNull) CTypeConversion.toJavaString(binPath) else "<unset>"
      System.err.println("madura: javac invocation failed (binPath: $bin)")
      err.printStackTrace()
      return 2
    }
  }
}
