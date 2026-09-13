package com.ciel.app;

import de.mkammerer.argon2.Argon2;
import de.mkammerer.argon2.Argon2Factory;
import org.springframework.stereotype.Component;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HexFormat;

@Component
public class CryptoService {
    private final Argon2 argon2 = Argon2Factory.create(Argon2Factory.Argon2Types.ARGON2id);

    public String hashPassword(String password) {
        char[] chars = password.toCharArray();
        try {
            return argon2.hash(2, 19456, 1, chars);
        } finally {
            argon2.wipeArray(chars);
        }
    }

    public boolean verifyPassword(String password, String phc) {
        char[] chars = password.toCharArray();
        try {
            return argon2.verify(phc, chars);
        } catch (Exception e) {
            return false;
        } finally {
            argon2.wipeArray(chars);
        }
    }

    public String sha256Hex(String value) {
        try {
            MessageDigest md = MessageDigest.getInstance("SHA-256");
            byte[] digest = md.digest(value.getBytes(StandardCharsets.UTF_8));
            return HexFormat.of().formatHex(digest);
        } catch (Exception e) {
            throw new IllegalStateException(e);
        }
    }
}
