package dev.elide.jvm;

import javax.tools.JavaCompiler
import javax.tools.ToolProvider
import javax.tools.JavaFileObject
import javax.tools.DiagnosticCollector
import java.io.File

object JvmInvoker {
  //
  @JvmStatic fun main(args: Array<String>) {
    // compiler
    val compiler = ToolProvider.getSystemJavaCompiler()
    if (compiler == null) {
      System.out.println("Failed to load compiler.")
      System.exit(2)
      return
    }

    // diag
    val diagnostics = DiagnosticCollector<JavaFileObject>()

    // exit
    System.out.println("Hello Kotlin Entry")
    System.exit(0)
  }
}
