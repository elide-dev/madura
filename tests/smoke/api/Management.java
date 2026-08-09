// madura: releases=8,17,25
package api;

import java.lang.management.ManagementFactory;
import java.lang.management.RuntimeMXBean;

/** `java.management`, likewise absent from the shipped image. */
public final class Management {
    private Management() {}

    public static long uptimeMillis() {
        RuntimeMXBean runtime = ManagementFactory.getRuntimeMXBean();
        return runtime.getUptime();
    }
}
