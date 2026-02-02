# Ciel End-to-End Encryption Implementation Plan

## Executive Summary

This document outlines a comprehensive plan to implement end-to-end encryption (E2EE) for Ciel, ensuring that only intended recipients can view media content. The solution draws from industry best practices, particularly Signal's proven architecture, while adapting to Ciel's social media use case.

## 1. Architecture Overview

### Core Principles
- **Zero Knowledge**: Server never sees unencrypted content or keys
- **Client-Side Encryption**: All encryption/decryption happens on user devices
- **Fine-Grained Access Control**: Per-post encryption keys with recipient-specific access
- **Password-Based Recovery**: Secure key backup tied to user credentials

### High-Level Architecture
```
[User Device] ←E2EE→ [Ciel Server] ←E2EE→ [Recipient Device]
    │                  │
    ├─ Generate keys   ├─ Store encrypted keys
    ├─ Encrypt media   ├─ Manage access control
    ├─ Decrypt media   └─ Distribute encrypted keys
    └─ Key management
```

## 2. Cryptographic Foundation

### Encryption Standards
- **Symmetric Encryption**: AES-256-GCM (for media files)
- **Asymmetric Encryption**: X25519 + Ed25519 (from Signal Protocol)
- **Key Derivation**: Argon2id (for password-based key protection)
- **Hashing**: SHA-256 for integrity checks

### Key Hierarchy
```
User Password
    ↓ (Argon2id)
Derived Key
    ↓ (AES-256-GCM)
Encrypted Private Key
    ↓ (Decrypt with password)
User Private Key (Ed25519)
    ↓ (Sign)
Media Encryption Key (AES-256)
    ↓ (Encrypt media)
Encrypted Media File
```

## 3. Database Schema Changes

### New Tables
```sql
-- User key management
CREATE TABLE user_crypto_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_key TEXT NOT NULL, -- Base64 encoded Ed25519 public key
    encrypted_private_key TEXT NOT NULL, -- AES-256-GCM encrypted private key
    key_derivation_salt TEXT NOT NULL, -- Base64 encoded salt
    key_derivation_iterations INT NOT NULL DEFAULT 100000,
    key_derivation_algorithm TEXT NOT NULL DEFAULT 'Argon2id',
    key_created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    key_last_rotated TIMESTAMPTZ,
    recovery_key_hash TEXT -- Hash of recovery code for key recovery
);

-- Media encryption keys (one per media file)
CREATE TABLE media_encryption_keys (
    media_id UUID PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
    encryption_key_encrypted TEXT NOT NULL, -- Encrypted with owner's public key
    encryption_algorithm TEXT NOT NULL DEFAULT 'AES-256-GCM',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Recipient access keys
CREATE TABLE media_recipient_keys (
    media_id UUID NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    encrypted_key TEXT NOT NULL, -- Media key encrypted with recipient's public key
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (media_id, recipient_id)
);

-- Key rotation history (for auditing)
CREATE TABLE key_rotation_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    old_key_fingerprint TEXT,
    new_key_fingerprint TEXT,
    rotated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reason TEXT
);
```

## 4. Implementation Phases

### Phase 1: Cryptographic Foundation (2-3 weeks)
**Objective**: Set up core cryptographic utilities and key management

**Tasks**:
1. **Add cryptographic dependencies**:
   ```toml
   # Cargo.toml additions
   [dependencies]
   ring = "0.16" # For cryptographic primitives
   argon2 = "0.5" # For password-based key derivation
   ed25519-dalek = "2.0" # For Signal-compatible keys
   aes-gcm = "0.10" # For AES-GCM encryption
   base64 = "0.21" # For key serialization
   ```

2. **Implement cryptographic utilities** (`src/infra/crypto.rs`):
   - Key generation (Ed25519/X25519)
   - Password-based key derivation (Argon2id)
   - AES-256-GCM encryption/decryption
   - Key serialization/deserialization

3. **Add user key management endpoints**:
   - `POST /api/v1/crypto/keys` - Generate new key pair
   - `GET /api/v1/crypto/keys` - Get public key and encrypted private key
   - `POST /api/v1/crypto/keys/rotate` - Rotate keys

4. **Implement password-based key backup**:
   - Client-side key encryption before storage
   - Server-side storage of encrypted keys
   - Key decryption on login

### Phase 2: Media Encryption (3-4 weeks)
**Objective**: Enable encrypted media uploads and storage

**Tasks**:
1. **Modify upload flow**:
   - Client generates random AES-256 key for each media file
   - Client encrypts media file before upload
   - Client encrypts AES key with their public key
   - Server stores encrypted media + encrypted key

2. **Update media service** (`src/app/media.rs`):
   ```rust
   pub struct EncryptedMediaUpload {
       pub upload_id: Uuid,
       pub object_key: String,
       pub upload_url: String,
       pub encryption_info: EncryptionInfo, // New field
   }

   pub struct EncryptionInfo {
       pub algorithm: String,
       pub key_encrypted: String, // Encrypted with user's public key
       pub iv: String, // Initialization vector
       pub auth_tag: String, // Authentication tag
   }
   ```

3. **Add media encryption endpoints**:
   - `POST /api/v1/media/encrypted` - Upload encrypted media metadata
   - `GET /api/v1/media/{id}/keys` - Get encryption keys for authorized users

4. **Implement key distribution**:
   - When post is created, encrypt media key for each eligible recipient
   - Store recipient keys in `media_recipient_keys` table

### Phase 3: Access Control & Sharing (2-3 weeks)
**Objective**: Implement fine-grained access control for encrypted media

**Tasks**:
1. **Modify post visibility logic**:
   - When post visibility changes, update recipient keys
   - Add/remove recipient keys as follow relationships change

2. **Implement key distribution service**:
   ```rust
   pub struct KeyDistributionService {
       db: Db,
       crypto: CryptoService,
   }

   impl KeyDistributionService {
       pub async fn share_media_with_followers(
           &self,
           media_id: Uuid,
           owner_id: Uuid,
       ) -> Result<()> {
           // Get all followers of owner
           // Encrypt media key with each follower's public key
           // Store in media_recipient_keys
       }

       pub async fn get_decryption_keys(
           &self,
           user_id: Uuid,
           media_id: Uuid,
       ) -> Result<Option<DecryptionKeys>> {
           // Check if user has access to media
           // Return encrypted key if authorized
       }
   }
   ```

3. **Update feed service** to include encryption keys:
   - Modify `get_home_feed` to return encryption keys for accessible media
   - Add client-side decryption capabilities

### Phase 4: Client Implementation (4-6 weeks, parallel)
**Objective**: Build frontend/client-side encryption capabilities

**Tasks**:
1. **Web Client (JavaScript/TypeScript)**:
   - Use Web Crypto API for browser-based encryption
   - Implement key management with IndexedDB storage
   - Add media encryption before upload
   - Implement media decryption after download

2. **Mobile Clients (Future)**:
   - Native encryption libraries for iOS/Android
   - Secure enclave storage for private keys
   - Background encryption for large files

3. **Fallback Mechanisms**:
   - Progressive enhancement for browsers without Web Crypto
   - Graceful degradation for unsupported devices

## 5. Signal Protocol Integration

### Components to Adopt from Signal

1. **Double Ratchet Algorithm**:
   - Provides forward secrecy and post-compromise security
   - Automatic key rotation for ongoing conversations

2. **X3DH Key Agreement**:
   - Extended Triple Diffie-Hellman for initial key exchange
   - Prevents man-in-the-middle attacks

3. **Session Management**:
   - Signal's session building and management
   - Handling of lost or compromised keys

### Implementation Approach

```rust
// src/infra/signal_protocol.rs
pub struct SignalProtocol {
    identity_key: IdentityKey,
    signed_pre_key: SignedPreKey,
    pre_keys: Vec<PreKey>,
    sessions: HashMap<Uuid, SessionState>,
}

impl SignalProtocol {
    pub fn new() -> Self {
        // Initialize with fresh key material
    }

    pub fn establish_session(&mut self, recipient_public_key: &PublicKey) -> SessionResult {
        // Implement X3DH key agreement
    }

    pub fn encrypt_message(&self, session_id: Uuid, plaintext: &[u8]) -> EncryptedMessage {
        // Implement Double Ratchet encryption
    }

    pub fn decrypt_message(&mut self, session_id: Uuid, ciphertext: &EncryptedMessage) -> DecryptionResult {
        // Implement Double Ratchet decryption
    }
}
```

### Benefits of Signal Protocol
- **Proven Security**: Used by billions of users
- **Forward Secrecy**: Compromised keys don't reveal past messages
- **Post-Compromise Security**: Key rotation limits damage from key theft
- **Interoperability**: Potential future compatibility with other Signal-based apps

## 6. Security Considerations

### Threat Model Mitigations

| Threat | Mitigation Strategy |
|--------|---------------------|
| Database breach | Encrypted keys useless without password |
| Server compromise | Forward secrecy limits exposure |
| Key theft | Automatic key rotation (Double Ratchet) |
| Brute force | Strong KDF (Argon2id) with high iterations |
| Man-in-the-middle | X3DH key agreement with identity verification |
| Lost devices | Password-based recovery + recovery codes |

### Security Best Practices

1. **Key Management**:
   - Never store unencrypted private keys on server
   - Use secure memory for temporary key storage
   - Implement proper key zeroization

2. **Cryptographic Agility**:
   - Version all encryption operations
   - Support multiple algorithms for migration
   - Regular algorithm updates

3. **Audit Logging**:
   - Log key rotations and access changes
   - Monitor for unusual access patterns
   - Alert on potential security events

## 7. User Experience Design

### Key User Flows

1. **First-Time Setup**:
   - Generate keys on device
   - Encrypt private key with password
   - Store recovery codes securely
   - Educate about key safety

2. **Media Upload**:
   - Automatic encryption (transparent to user)
   - Progress indicators for encryption + upload
   - Clear security indicators

3. **Media Viewing**:
   - Seamless decryption (if authorized)
   - Clear error messages for access denied
   - Loading states for decryption

4. **Key Recovery**:
   - Password-based recovery flow
   - Recovery code fallback
   - Multi-factor verification

### Security Indicators
- ✅ **Green lock**: Fully encrypted and verified
- ⚠️ **Yellow shield**: Encrypted but unverified
- ❌ **Red warning**: Encryption issues or access denied

## 8. Migration Strategy

### From Unencrypted to Encrypted

1. **Phase 1: Opt-In (2 months)**:
   - Allow users to enable encryption
   - New posts can be encrypted
   - Educate users about benefits

2. **Phase 2: Default Encryption (1 month)**:
   - New accounts get encryption by default
   - Existing users prompted to enable
   - Provide migration tools

3. **Phase 3: Full Encryption (3 months)**:
   - All new media encrypted
   - Legacy media gradually re-encrypted
   - Final cutoff for unencrypted content

### Backward Compatibility
- Support both encrypted and unencrypted media during transition
- Clear indicators for encryption status
- Migration tools for existing content

## 9. Performance Optimization

### Encryption Performance
- **Web Workers**: Offload encryption to background threads
- **Chunked Processing**: Encrypt large files in chunks
- **Progressive Loading**: Decrypt and display as data arrives

### Storage Optimization
- **Compression**: Compress before encryption
- **Deduplication**: Identify and eliminate duplicate encrypted files
- **CDN Caching**: Cache encrypted files at edge locations

### Network Optimization
- **Delta Updates**: Only transmit changed encryption keys
- **Batch Operations**: Group key distribution operations
- **Lazy Loading**: Decrypt media only when needed

## 10. Monitoring and Maintenance

### Key Metrics to Track
- Encryption/decryption success rates
- Key rotation frequency
- Access control errors
- Performance metrics (encryption time, etc.)
- User adoption rates

### Maintenance Tasks
- Regular key rotation for all users
- Algorithm updates and migrations
- Security audits and penetration testing
- User education and support

## 11. Legal and Compliance

### Data Protection Regulations
- **GDPR**: Right to be forgotten, data minimization
- **CCPA**: User data rights and disclosures
- **Global Compliance**: Jurisdiction-specific requirements

### Law Enforcement Considerations
- **Warrant Canary**: Transparency about government requests
- **No Backdoors**: Commitment to user privacy
- **Legal Process**: Clear policies for data requests

## 12. Implementation Timeline

| Phase | Duration | Key Deliverables |
|-------|----------|-------------------|
| 1. Foundation | 2-3 weeks | Crypto utilities, key management |
| 2. Media Encryption | 3-4 weeks | Encrypted uploads, key storage |
| 3. Access Control | 2-3 weeks | Key distribution, sharing logic |
| 4. Client Implementation | 4-6 weeks | Web/mobile encryption clients |
| 5. Signal Integration | 3-4 weeks | Double Ratchet, X3DH implementation |
| 6. Testing & Security Audit | 4 weeks | Penetration testing, code audit |
| 7. Beta Rollout | 2 weeks | Limited user testing |
| 8. Full Launch | 1 week | Production deployment |

## 13. Risk Assessment

### Technical Risks
- **Performance**: Encryption overhead on mobile devices
- **Compatibility**: Browser/device support limitations
- **Complexity**: Increased system complexity

### User Risks
- **Key Loss**: Users losing access to their data
- **Usability**: Complexity may deter some users
- **Expectations**: Users may not understand limitations

### Mitigation Strategies
- **Progressive Enhancement**: Graceful degradation
- **Education**: Clear documentation and tutorials
- **Support**: Dedicated help resources
- **Backups**: Multiple recovery options

## 14. Success Metrics

### Technical Success
- 99.9% encryption/decryption success rate
- <100ms encryption time for typical images
- <500ms key distribution time
- Zero security incidents during beta

### User Adoption
- 80%+ of active users enable encryption within 3 months
- 95%+ of new content encrypted within 6 months
- <5% user-reported issues with encryption

### Business Impact
- Increased user trust and retention
- Competitive differentiation
- Positive press coverage
- Reduced liability from data breaches

## 15. Resources and References

### Libraries and Tools
- **Signal Protocol**: https://signal.org/docs/
- **Libsignal (Rust)**: https://github.com/signalapp/libsignal
- **Web Crypto API**: https://developer.mozilla.org/en-US/docs/Web/API/Web_Crypto_API
- **Argon2**: https://github.com/P-H-C/phc-winner-argon2

### Standards and RFCs
- RFC 7748: Elliptic Curves for Security
- RFC 5869: HMAC-based Extract-and-Expand Key Derivation
- NIST SP 800-185: SHA-3 Derived Functions

### Security Audits
- Signal Protocol Audit: https://eprint.iacr.org/2016/1013.pdf
- Matrix Olm Audit: https://matrix.org/blog/2016/11/21/matrixs-olfm-end-to-end-encryption-security-assessment-released-and-implemented/

## Conclusion

This plan outlines a comprehensive approach to implementing end-to-end encryption in Ciel, drawing from the best practices of industry leaders like Signal while adapting to the specific needs of a social media platform. The phased implementation allows for gradual rollout and testing, while the integration of proven protocols like Signal's Double Ratchet ensures robust security.

The successful implementation of this plan will position Ciel as a leader in user privacy, providing true end-to-end encryption where even the platform operator cannot access user content, while maintaining a user-friendly experience and robust recovery options.