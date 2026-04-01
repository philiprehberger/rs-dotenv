# rs-dotenv

[![CI](https://github.com/philiprehberger/rs-dotenv/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-dotenv/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-dotenv.svg)](https://crates.io/crates/philiprehberger-dotenv)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-dotenv)](https://github.com/philiprehberger/rs-dotenv/commits/main)

Fast .env file parser with variable interpolation, multi-file layering, and type-safe loading

## Installation

```toml
[dependencies]
philiprehberger-dotenv = "0.1.1"
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

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-dotenv)

🐛 [Report issues](https://github.com/philiprehberger/rs-dotenv/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-dotenv/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
