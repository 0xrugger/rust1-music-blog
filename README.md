# Music Blog: A Study in HTTP Fundamentals
This was my first Rust project – raw, unpolished, and intentionally built without frameworks to grasp what happens beneath the surface.

## ⚠️ First Rust Project Disclaimer

This repository represents my **first experience with Rust**. Expect:
- Non-idiomatic code patterns
- Suboptimal error handling
- Reinvented wheels
- Learning curve artifacts

This isn't a showcase of Rust best practices – it's a documentation of the learning process itself. Every mistake taught me something about ownership, the borrow checker, and why Rust's safety guarantees matter.

## 🎯 Philosophy: Zero-Dependency Control

**Core principle:** No web frameworks. No magic. Only `std` and absolutely minimal crates (Askama for templates). This is a conscious rejection of abstractions to understand the protocol's physics.

| Component | Choice | Why |
|-----------|--------|-----|
| Web Server | Raw TCP | Understand request/response wire format |
| HTTP Parser | Custom | Control every byte, no hidden behaviors |
| Multipart Parser | Custom | Learn why file uploads fail in production |
| Routing | Manual | See how frameworks normalize paths |
| Async | None | Master sync before diving into async complexity |

## 🏗️ Architecture Layers

### Layer 1: Raw TCP Socket
The server manually binds a `TcpListener` and parses incoming byte streams. No `hyper`, no `axum` – just bytes in, bytes out.

### Layer 2: Custom HTTP Parser
Manual header parsing, method extraction, path routing. Implemented basic routes:
- `/` – Homepage with all posts
- `/upload` – Post creation form
- `/posts/{slug}` – Individual post view
- `/static/` – Asset serving

### Layer 3: Custom Multipart Parser ⚡
The hardest part. Manual `multipart/form-data` parsing without libraries:
- Boundary detection and validation
- File and field extraction
- Content-type validation
- Size limiting

**Why?** If you can't parse multipart by hand, you'll never understand why production uploads fail at scale.

### Layer 4: Business Logic & State
- **PRG Pattern (Post/Redirect/Get)** – Manual implementation for idempotency
- **Filesystem as Database** – Posts stored as `.md` files with metadata
- **Image Validation** – Manual JPG/PNG signature checking (no `image-rs`)
- **Dynamic Navigation** – Content generation based on filesystem state

### Layer 5: UI & Templates
- **Askama** – Type-safe templates
- **CSS Evolution** – Reduced from 700 to 230 lines through deliberate refactoring
- **Mobile First** – Responsive design with dark/light themes

## 🚀 Running Locally

```bash
# Clone the repository
git clone https://github.com/0xrugger/rust1-music-blog
cd rust1-music-blog

# Create necessary directories
mkdir -p posts images

# Run the server
cargo run

# Visit http://localhost:7878
