// madura: releases=21-25
package release;

/**
 * Sealed types and pattern-matching switch, final in 21. Compiles to several
 * class files, so it also covers a nested-output compile.
 */
public sealed interface Shape permits Shape.Circle, Shape.Square {
    record Circle(double radius) implements Shape {}

    record Square(double side) implements Shape {}

    static double area(Shape shape) {
        return switch (shape) {
            case Circle circle -> Math.PI * circle.radius() * circle.radius();
            case Square square -> square.side() * square.side();
        };
    }
}
