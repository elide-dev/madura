package dev.elide.jvm

import java.io.OutputStream
import java.io.PrintStream

class CapturedStreams private constructor (activate: Boolean = true) : AutoCloseable {
  private val originalOut = System.`out`
  private val originalErr = System.`err`
  private val emptyStream = PrintStream(OutputStream.nullOutputStream())

  private fun activate() {
    System.setOut(emptyStream)
    System.setErr(emptyStream)
  }

  private fun restore() {
    System.setOut(originalOut)
    System.setErr(originalErr)
  }

  init {
    if (activate) {
      activate()
    }
  }

  override fun close() {
    restore()
  }

  companion object {
    @JvmStatic fun withCaptured(op: () -> Unit) {
      CapturedStreams(activate = true).use {
        op.invoke()
      }
    }
  }
}
