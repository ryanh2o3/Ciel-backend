package com.ciel.web;

import java.math.BigInteger;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Parses {@code TRUSTED_PROXY_CIDRS} and tests peer membership, matching Rust's
 * {@code trusted_proxy_cidrs} / {@code peer_in_trusted_proxies}.
 */
public final class TrustedProxies {

    public record Cidr(InetAddress network, int prefixLength) {
        boolean contains(InetAddress addr) {
            byte[] net = network.getAddress();
            byte[] ip = addr.getAddress();
            if (net.length != ip.length) {
                return false;
            }
            BigInteger netBi = new BigInteger(1, net);
            BigInteger ipBi = new BigInteger(1, ip);
            int bits = net.length * 8;
            int shift = bits - prefixLength;
            if (shift < 0) {
                return false;
            }
            return netBi.shiftRight(shift).equals(ipBi.shiftRight(shift));
        }
    }

    private TrustedProxies() {}

    public static List<Cidr> parse(String raw) {
        if (raw == null || raw.isBlank()) {
            return List.of();
        }
        List<Cidr> out = new ArrayList<>();
        for (String part : raw.split(",")) {
            String trimmed = part.trim();
            if (trimmed.isEmpty()) {
                continue;
            }
            out.add(parseOne(trimmed));
        }
        return Collections.unmodifiableList(out);
    }

    private static Cidr parseOne(String cidr) {
        String[] bits = cidr.split("/", 2);
        try {
            InetAddress network = InetAddress.getByName(bits[0]);
            int prefix = bits.length == 2
                    ? Integer.parseInt(bits[1])
                    : network.getAddress().length * 8;
            int max = network.getAddress().length * 8;
            if (prefix < 0 || prefix > max) {
                throw new IllegalArgumentException("prefix out of range: " + cidr);
            }
            return new Cidr(network, prefix);
        } catch (UnknownHostException | NumberFormatException e) {
            throw new IllegalArgumentException("invalid CIDR in TRUSTED_PROXY_CIDRS: " + cidr, e);
        }
    }

    public static boolean contains(List<Cidr> cidrs, String remoteAddr) {
        if (cidrs == null || cidrs.isEmpty() || remoteAddr == null || remoteAddr.isBlank()) {
            return false;
        }
        try {
            InetAddress addr = InetAddress.getByName(stripZoneId(remoteAddr));
            for (Cidr cidr : cidrs) {
                if (cidr.contains(addr)) {
                    return true;
                }
            }
            return false;
        } catch (UnknownHostException e) {
            return false;
        }
    }

    /** Strip IPv6 zone id (e.g. {@code fe80::1%eth0}). */
    private static String stripZoneId(String addr) {
        int pct = addr.indexOf('%');
        return pct >= 0 ? addr.substring(0, pct) : addr;
    }
}
