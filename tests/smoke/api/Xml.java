// madura: releases=8,17,25
package api;

import javax.xml.parsers.DocumentBuilderFactory;
import javax.xml.parsers.ParserConfigurationException;

/** `java.xml`, likewise absent from the shipped image. */
public final class Xml {
    private Xml() {}

    public static DocumentBuilderFactory hardened() throws ParserConfigurationException {
        DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
        factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
        factory.setExpandEntityReferences(false);
        return factory;
    }
}
