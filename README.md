# 🪣 FreeBucket

**A local S3-compatible object storage service built with Rust.**

FreeBucket provides a lightweight, fast, and beautiful local alternative to cloud object storage services like AWS S3, DigitalOcean Spaces, or MinIO. Perfect for development, testing, and local file management.

---

## ✨ Features

- **S3-Compatible API** — Familiar PUT/GET/DELETE operations on buckets and objects
- **REST API** — Clean JSON-based API for all operations
- **Web Dashboard** — Beautiful dark-mode UI for managing buckets and objects
- **Drag & Drop Upload** — Upload files directly from the browser
- **Filesystem Backend** — All data stored as regular files on your local disk
- **Metadata Tracking** — Content types, ETags, custom metadata (x-amz-meta-*)
- **Prefix/Delimiter Listing** — S3-style folder simulation
- **Zero Configuration** — Just run it and go

## 🚀 Quick Start

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) 1.70+

### Build & Run

```bash
cargo run
```

The server starts on `http://127.0.0.1:3210` by default.

### Configuration (Environment Variables)

| Variable | Default | Description |
|---|---|---|
| `FREEBUCKET_HOST` | `127.0.0.1` | Host to bind to |
| `FREEBUCKET_PORT` | `3210` | Port to listen on |
| `FREEBUCKET_DATA_DIR` | `./freebucket_data` | Directory for stored data |

## 📡 API Reference

### Buckets

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/buckets` | List all buckets |
| `POST` | `/api/buckets` | Create a new bucket |
| `GET` | `/api/buckets/{name}` | Get bucket details |
| `DELETE` | `/api/buckets/{name}` | Delete a bucket |

### Objects

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/buckets/{bucket}/objects` | List objects |
| `GET` | `/api/buckets/{bucket}/objects/{key}` | Download an object |
| `POST` | `/api/buckets/{bucket}/upload` | Upload via multipart |
| `DELETE` | `/api/buckets/{bucket}/objects/{key}` | Delete an object |

### S3-Compatible Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/s3/` | List buckets |
| `PUT` | `/s3/{bucket}` | Create bucket |
| `DELETE` | `/s3/{bucket}` | Delete bucket |
| `GET` | `/s3/{bucket}?prefix=...&delimiter=...` | List objects |
| `PUT` | `/s3/{bucket}/{key}` | Upload object |
| `GET` | `/s3/{bucket}/{key}` | Download object |
| `DELETE` | `/s3/{bucket}/{key}` | Delete object |

### Stats

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/stats` | Get storage statistics |

## 💡 Usage Examples

### Create a Bucket

```bash
curl -X POST http://localhost:3210/api/buckets \
  -H "Content-Type: application/json" \
  -d '{"name": "my-bucket", "region": "local"}'
```

### Upload a File

```bash
curl -X POST http://localhost:3210/api/buckets/my-bucket/upload \
  -F "file=@photo.jpg"
```

### Upload via S3-Compatible API

```bash
curl -X PUT http://localhost:3210/s3/my-bucket/docs/readme.txt \
  -H "Content-Type: text/plain" \
  -d "Hello, FreeBucket!"
```

### Download a File

```bash
curl http://localhost:3210/api/buckets/my-bucket/objects/photo.jpg -o photo.jpg
```

### List Objects with Prefix

```bash
curl "http://localhost:3210/api/buckets/my-bucket/objects?prefix=docs/&delimiter=/"
```

### Delete an Object

```bash
curl -X DELETE http://localhost:3210/api/buckets/my-bucket/objects/photo.jpg
```

## 🏗️ Architecture

```
freebucket_data/
├── my-bucket/
│   ├── .bucket_meta.json      # Bucket metadata
│   ├── .meta/                 # Object metadata files
│   │   ├── photo.jpg.json
│   │   └── docs__SLASH__readme.txt.json
│   └── objects/               # Actual object data
│       ├── photo.jpg
│       └── docs/
│           └── readme.txt
└── another-bucket/
    ├── ...
```

## 📜 License

MIT
