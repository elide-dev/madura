package dev.elide.jvm;

import javax.tools.ToolProvider

object JvmInvoker {
  @JvmStatic fun main(args: Array<String>) {
    val compiler = ToolProvider.getSystemJavaCompiler()
    if (compiler == null) {
      System.err.println("madura: system Java compiler is not available in this image")
      System.exit(2)
      return
    }
    System.exit(compiler.run(System.`in`, System.out, System.err, *args))
  }
}

