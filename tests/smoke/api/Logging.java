// madura: releases=8,17,25
package api;

import java.util.logging.Level;
import java.util.logging.Logger;

/**
 * `java.logging` is not in the platform image madura ships, so this compiles
 * only because `--release` resolves the API against the bundled `ct.sym`.
 */
public final class Logging {
    private static final Logger LOG = Logger.getLogger("madura.smoke");

    private Logging() {}

    public static void emit(String message) {
        LOG.log(Level.INFO, message);
    }
}
