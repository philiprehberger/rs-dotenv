# rs-dotenv

Fast .env file parser with variable interpolation, multi-file layering, and type-safe loading.

## Installation

```toml
[dependencies]
philiprehberger-dotenv = "0.1"
```

## Usage

```rust
use philiprehberger_dotenv::DotEnv;

// Load .env from current directory
let env = DotEnv::load()?;

// Type-safe access
let port: u16 = env.get_as("PORT")?;
let debug: bool = env.get_bool("DEBUG")?;
let name = env.get_or("APP_NAME", "my-app");

// Validate required variables
env.require(&["DATABASE_URL", "SECRET_KEY"])?;
```

### Multi-file layering

```rust
use philiprehberger_dotenv::DotEnv;

// Later files override earlier ones
let env = DotEnv::load_layered(&[".env", ".env.local"])?;
```

### Variable interpolation

```env
DB_HOST=localhost
DB_PORT=5432
DATABASE_URL=postgres://${DB_HOST}:${DB_PORT}/mydb
```

### Load into process environment

```rust
use philiprehberger_dotenv;

// Load .env and set all vars into process environment
philiprehberger_dotenv::load_and_apply()?;
```

## API

| Function / Type | Description |
|----------------|-------------|
| `DotEnv::load()` | Load `.env` from current directory |
| `DotEnv::load_from(path)` | Load from specific file |
| `DotEnv::load_layered(paths)` | Load multiple files with priority |
| `.get(key)` | Get raw string value |
| `.get_or(key, default)` | Get with default |
| `.get_as::<T>(key)` | Type-safe parsing |
| `.get_bool(key)` | Parse boolean values |
| `.get_list(key, sep)` | Split value into list |
| `.require(keys)` | Validate required variables |
| `.apply()` | Set vars into process environment |
| `load_and_apply()` | Load .env and apply to process |

## License

MIT
