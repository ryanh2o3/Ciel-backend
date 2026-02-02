# Ciel: Anti-Bot, Referral, and Sharing Strategy

**Goals:**

- Minimize platform abuse (bots, scammers, spam).
- Avoid mandatory ID verification.
- Keep costs low and architecture scalable.
- Ensure a seamless, frictionless experience for real users.

---

## **1. Anti-Bot and Abuse Prevention**

### **A. Device and Behavior Fingerprinting**

- **Tool:** [FingerprintJS](https://github.com/fingerprintjs/fingerprintjs)
- **How it works:**
  - Generate a device hash during signup/login.
  - Track anomalies (e.g., multiple accounts from the same device).
- **Implementation:**
  - Store hashes in Redis/PostgreSQL.
  - Rate-limit or block suspicious devices.

### **B. Proof-of-Work (PoW) for Signups**

- **Tool:** Custom Hashcash implementation
- **How it works:**
  - Require a small computational task during signup.
  - Bots struggle to scale; real users experience minimal delay.
- **Implementation:**
  - Integrate into the signup flow (frontend + backend validation).

### **C. Rate Limiting and Trust Tiers**

- **How it works:**
  - New accounts: 1 post/hour, 5 follows/day.
  - Trusted accounts: Higher limits after 7 days.
- **Implementation:**
  - Use Redis for rate limiting.
  - Adjust limits based on trust scores.

### **D. Content and Behavior Moderation**

- **Tools:**
  - [TensorFlow Lite](https://www.tensorflow.org/lite) for image moderation.
  - PostgreSQL for tracking follow/like patterns.
- **How it works:**
  - Scan uploads for NSFW/duplicate content.
  - Flag accounts with abnormal behavior (e.g., bulk follows).

### **E. Social Graph Analysis**

- **How it works:**
  - Monitor follow/like patterns for bot-like activity.
  - Example: 1,000 follows in 1 hour → Flag for review.

---

## **2. Referral-Only Signup System**

### **A. One-Time-Use Invite Codes**

- **How it works:**
  - Each invite code is unique and expires after 7 days.
  - Codes are invalidated after use.
- **Database Schema (PostgreSQL):**
  ```sql
  CREATE TABLE invites (
    code VARCHAR(16) PRIMARY KEY,
    created_by INTEGER REFERENCES users(id),
    used_by INTEGER REFERENCES users(id),
    created_at TIMESTAMP DEFAULT NOW(),
    used_at TIMESTAMP,
    expires_at TIMESTAMP,
    is_valid BOOLEAN DEFAULT TRUE
  );
  ```
