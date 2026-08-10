// madura: releases=8,17,25
package api;

import java.sql.SQLException;
import java.sql.Types;

/** `java.sql`, likewise absent from the shipped image. */
public final class Sql {
    private Sql() {}

    public static boolean isTimeout(SQLException error) {
        return "HYT00".equals(error.getSQLState());
    }

    public static int varchar() {
        return Types.VARCHAR;
    }
}
