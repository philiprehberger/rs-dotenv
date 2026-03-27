# rs-dotenv

[![CI](https://github.com/philiprehberger/rs-dotenv/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-dotenv/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-dotenv.svg)](https://crates.io/crates/philiprehberger-dotenv)
[![GitHub release](https://img.shields.io/github/v/release/philiprehberger/rs-dotenv)](https://github.com/philiprehberger/rs-dotenv/releases)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-dotenv)](https://github.com/philiprehberger/rs-dotenv/commits/main)
[![License](https://img.shields.io/github/license/philiprehberger/rs-dotenv)](LICENSE)
[![Bug Reports](https://img.shields.io/github/issues/philiprehberger/rs-dotenv/bug)](https://github.com/philiprehberger/rs-dotenv/issues?q=is%3Aissue+is%3Aopen+label%3Abug)
[![Feature Requests](https://img.shields.io/github/issues/philiprehberger/rs-dotenv/enhancement)](https://github.com/philiprehberger/rs-dotenv/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)
[![Sponsor](https://img.shields.io/badge/sponsor-GitHub%20Sponsors-ec6cb9)](https://github.com/sponsors/philiprehberger)

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

If you find this package useful, consider giving it a star on GitHub — it helps motivate continued maintenance and development.

[![LinkedIn](https://img.shields.io/badge/Philip%20Rehberger-LinkedIn-0A66C2?logo=linkedin)](https://www.linkedin.com/in/philiprehberger)
[![More packages](https://img.shields.io/badge/more-open%20source%20packages-blue)](https://philiprehberger.com/open-source-packages)

## License

[MIT](LICENSE)
