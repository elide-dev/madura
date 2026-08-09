// madura: releases=8-25
package release;

import java.util.ArrayList;
import java.util.List;

/**
 * Java 8 source, compiled at every release madura claims to support. Nothing
 * here is newer than 8, so a failure at any release is a toolchain problem
 * rather than a language-level one.
 */
public final class Baseline {
    private Baseline() {}

    public static List<String> shout(List<String> words) {
        List<String> out = new ArrayList<String>();
        for (String word : words) {
            out.add(word.toUpperCase());
        }
        return out;
    }
}
