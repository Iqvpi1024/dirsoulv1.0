# Skill: Rust Memory Safety

> **Purpose**: Ensure Rust code maintains memory safety and handles concurrency correctly in the 8GB environment, especially when processing RawMemory with encrypted data.

---

## Core Principles

### Ownership Rules (Declarative Patterns)

```rust
// ✅ CORRECT: Clear ownership transfer
fn process_memory(input: RawMemory) -> Result<ProcessedMemory> {
    // input is owned here, moved into the function
    let encrypted = encrypt(&input.content)?;
    Ok(ProcessedMemory { encrypted })
}

// ❌ AVOID: Unnecessary cloning
fn process_memory(input: &RawMemory) -> Result<ProcessedMemory> {
    // Borrowing when ownership is clearer
    let encrypted = encrypt(&input.content)?;
    Ok(ProcessedMemory { encrypted })
}
```

### Borrowing Checker Patterns

```rust
// ✅ Multiple immutable borrows are valid
fn validate_memories(memories: &[RawMemory]) -> bool {
    for m in memories {
        validate_content(&m.content);  // &m borrowed here
        check_timestamp(&m.created_at); // &m borrowed again - OK!
    }
    true
}

// ❌ Mutable borrow conflicts
fn mutate_memories(memories: &mut [RawMemory]) -> bool {
    for m in memories.iter_mut() {
        let content = &m.content;      // immutable borrow
        m.encrypted = Some(encrypt(content)?); // ERROR! mutable borrow
    }
    true
}
```

### Lifetime Annotations for Memory Operations

```rust
// ✅ Explicit lifetime for reference validity
fn extract_from_memory<'a>(memory: &'a RawMemory) -> Event<'a> {
    Event {
        action: extract_action(&memory.content),
        source: &memory.content,  // Valid because 'a ties them together
    }
}

// ✅ Static lifetime for configuration
fn get_encryption_key() -> &'static [u8] {
    b"constant_key_for_example"  // Valid for entire program
}
```

---

## 8GB Memory Optimization Patterns

### Avoid Vec Over-Allocation

```rust
// ❌ AVOID: Unbounded growth
fn collect_all_memories() -> Vec<RawMemory> {
    let mut all = Vec::new();
    for chunk in db.iter() {
        all.extend(chunk);  // Could grow to millions of entries
    }
    all
}

// ✅ PREFER: Streaming with iterators
fn process_all_memories() -> impl Iterator<Item = RawMemory> {
    db.iter().filter_map(|m| m.ok())
}

// ✅ OR: Batch processing with known capacity
fn process_batch(batch_size: usize) -> Vec<ProcessedMemory> {
    let mut results = Vec::with_capacity(batch_size);
    for m in db.iter().take(batch_size) {
        if let Ok(memory) = m {
            results.push(process_memory(memory));
        }
    }
    results
}
```

### Use Box/Arc for Large Data

```rust
// ✅ Use Box for single large ownership
struct EncryptedContent {
    data: Box<[u8]>,  // Heap-allocated, single owner
}

// ✅ Use Arc for shared large data
struct SharedMemory {
    content: Arc<Vec<u8>>,  // Multiple references, one allocation
}

// ✅ Use Cow for conditional ownership
use std::borrow::Cow;

fn maybe_encrypt(data: Cow<[u8]>) -> Cow<[u8]> {
    if is_encrypted(&data) {
        data  // Return borrowed, no copy
    } else {
        Cow::Owned(encrypt(&data).into())  // Only encrypt if needed
    }
}
```

### BYTEA Encrypted Data Handling

```rust
// ✅ Safe pattern for processing encrypted BYTEA from PostgreSQL
use diesel::sql_types::Bytea;

struct RawMemory {
    encrypted: Option<Vec<u8>>,  // BYTEA from Postgres
}

fn decrypt_safe(memory: &RawMemory) -> Result<Vec<u8>> {
    match &memory.encrypted {
        Some(data) if !data.is_empty() => {
            // Decrypt in place, validate size first
            if data.len() < 32 {  // Fernet minimum size
                return Err(Error::InvalidData);
            }
            decrypt_fernet(data)
        }
        _ => Err(Error::NotEncrypted),
    }
}

// ❌ AVOID: Panic on large data
fn decrypt_unsafe(memory: &RawMemory) -> Vec<u8> {
    memory.encrypted.as_ref().unwrap().clone()  // Could panic!
}
```

---

## Common Pitfalls

### 1. Circular References

```rust
// ❌ DANGER: Reference cycle causes memory leak
use std::rc::Rc;
use std::cell::RefCell;

struct MemoryNode {
    parent: Option<Rc<RefCell<MemoryNode>>>,
    children: Vec<Rc<RefCell<MemoryNode>>>,
}

// ✅ USE: Weak references for back-links
use std::rc::{Rc, Weak};

struct MemoryNode {
    parent: Option<Weak<RefCell<MemoryNode>>>,  // Weak doesn't hold
    children: Vec<Rc<RefCell<MemoryNode>>>,
}
```

### 2. Tokio Async Send/Sync Requirements

```rust
// ❌ ERROR: Rc cannot be sent across threads
async fn process_memory_async(memory: Rc<RawMemory>) -> Result<()> {
    tokio::spawn(async move {
        analyze(memory)  // ERROR: Rc is not Send
    }).await?;
    Ok(())
}

// ✅ CORRECT: Use Arc for async sharing
async fn process_memory_async(memory: Arc<RawMemory>) -> Result<()> {
    tokio::spawn(async move {
        analyze(memory)  // OK: Arc is Send + Sync
    }).await?;
    Ok(())
}

// ✅ CORRECT: Clone for independent ownership
async fn process_memory_async(memory: RawMemory) -> Result<()> {
    tokio::spawn(async move {
        analyze(memory)  // OK: value moved
    }).await?;
    Ok(())
}
```

### 3. String vs &str Overhead

```rust
// ❌ Unnecessary String allocation
fn find_action(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        if is_action(word) {
            return Some(word.to_string());  // Allocates new String
        }
    }
    None
}

// ✅ Return borrowed slice
fn find_action(text: &str) -> Option<&str> {
    text.split_whitespace().find(|w| is_action(w))
}

// ✅ Use Cow for conditional allocation
use std::borrow::Cow;

fn maybe_uppercase(s: &str) -> Cow<str> {
    if s.chars().all(|c| c.is_uppercase()) {
        Cow::Borrowed(s)  // No allocation
    } else {
        Cow::Owned(s.to_uppercase())  // Allocate only when needed
    }
}
```

---

## Integration with Encryption

```rust
// ✅ Zero-copy encryption pattern
use std::io::Cursor;

fn encrypt_stream(data: &[u8]) -> Result<Vec<u8>> {
    let cursor = Cursor::new(data);
    let mut encryptor = FernetEncryptor::new();
    encryptor.update(cursor)?;  // Process without full copy
    encryptor.finalize()
}

// ✅ Chunked processing for large data
fn encrypt_large(data: &[u8], chunk_size: usize) -> Result<Vec<u8>> {
    data.chunks(chunk_size)
        .map(|chunk| encrypt_chunk(chunk))
        .collect()
}
```

---

## Memory Profiling Checklist

When writing memory-critical code for 8GB environment:

- [ ] Can I use iterators instead of collecting into Vec?
- [ ] Can I borrow instead of cloning?
- [ ] Can I use Cow for conditional ownership?
- [ ] Am I holding data longer than necessary?
- [ ] Can I process in chunks instead of loading all?
- [ ] Am I using Arc for shared read-only data?
- [ ] Can I use Box<[T]> instead of Vec<T> for fixed sizes?
- [ ] Are async closures Send + Sync?

---

## Recommended Combinations

Use this skill together with:
- **EncryptionBestPractices**: For secure RawMemory encryption
- **PostgresSchemaDesign**: For understanding BYTEA storage patterns
- **TestingAndDebugging**: For memory leak detection in tests
