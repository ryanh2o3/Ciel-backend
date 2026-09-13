package com.ciel.app;

import de.mkammerer.argon2.Argon2;
import de.mkammerer.argon2.Argon2Factory;
import jakarta.annotation.PreDestroy;
import org.springframework.stereotype.Component;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicInteger;

@Component
public class CryptoService {

    private static final long ARGON2_TIMEOUT_SECONDS = 10;

    private final Argon2 argon2 = Argon2Factory.create(Argon2Factory.Argon2Types.ARGON2id);

    /**
     * Argon2 hashing/verification burns tens of milliseconds of pure CPU per
     * call. Running it on a small dedicated pool (mirroring Rust's
     * {@code tokio::task::spawn_blocking} usage in {@code app/auth.rs}) bounds
     * how much CPU concurrent logins/signups can consume and keeps a single
     * slow hash from starving the Tomcat request-handling threads.
     */
    private final ExecutorService cryptoExecutor = Executors.newFixedThreadPool(4, new AuthWorkerThreadFactory());

    @PreDestroy
    void shutdown() {
        cryptoExecutor.shutdown();
    }

    public String hashPassword(String password) {
        char[] chars = password.toCharArray();
        try {
            return runBounded(() -> argon2.hash(2, 19456, 1, chars));
        } finally {
            argon2.wipeArray(chars);
        }
    }

    public boolean verifyPassword(String password, String phc) {
        char[] chars = password.toCharArray();
        try {
            return runBounded(() -> {
                try {
                    return argon2.verify(phc, chars);
                } catch (Exception e) {
                    return false;
                }
            });
        } catch (Exception e) {
            // Treat worker/timeout failures as invalid credentials rather than
            // surfacing a 500 to the client (matches Rust's verify_password).
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

    private <T> T runBounded(java.util.function.Supplier<T> work) {
        try {
            return CompletableFuture.supplyAsync(work, cryptoExecutor)
                    .get(ARGON2_TIMEOUT_SECONDS, TimeUnit.SECONDS);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new IllegalStateException("password hashing task interrupted", e);
        } catch (ExecutionException e) {
            throw new IllegalStateException("password hashing task failed", e.getCause());
        } catch (TimeoutException e) {
            throw new IllegalStateException("password hashing task timed out", e);
        }
    }

    private static final class AuthWorkerThreadFactory implements java.util.concurrent.ThreadFactory {
        private final AtomicInteger counter = new AtomicInteger();

        @Override
        public Thread newThread(Runnable r) {
            Thread t = new Thread(r, "argon2-worker-" + counter.incrementAndGet());
            t.setDaemon(true);
            return t;
        }
    }
}
