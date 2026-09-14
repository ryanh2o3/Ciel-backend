package com.ciel.web;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class TrustedProxiesTest {

    @Test
    void emptyMeansUntrusted() {
        assertFalse(TrustedProxies.contains(List.of(), "10.42.0.1"));
        assertFalse(TrustedProxies.contains(TrustedProxies.parse(""), "10.42.0.1"));
    }

    @Test
    void privateRanges() {
        var cidrs = TrustedProxies.parse("10.0.0.0/8,172.16.0.0/12,192.168.0.0/16");
        assertTrue(TrustedProxies.contains(cidrs, "10.42.0.1"));
        assertTrue(TrustedProxies.contains(cidrs, "172.21.0.2"));
        assertTrue(TrustedProxies.contains(cidrs, "192.168.1.10"));
        assertFalse(TrustedProxies.contains(cidrs, "8.8.8.8"));
    }
}
