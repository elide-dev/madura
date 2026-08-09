// madura: releases=17-25
package release;

/** Records: a language feature the class file format itself had to grow for. */
public record Point(int x, int y) {
    public int sum() {
        return x + y;
    }
}
