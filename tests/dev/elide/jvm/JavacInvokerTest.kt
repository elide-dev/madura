package dev.elide.jvm

import kotlin.test.*

class JavacInvokerTest {
  @Test fun testMainHelp() = CapturedStreams.withCaptured {
    assertEquals(0, JavacInvoker.compileMain(["-help"]))
  }

  @Test fun testMainBadFlags() = CapturedStreams.withCaptured {
    assertNotEquals(0, JavacInvoker.compileMain(["-notreal"]))
  }
}
