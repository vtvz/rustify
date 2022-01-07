set dotenv-load := false

generate:
  sea-orm-cli generate entity -o src/entity --expanded-format --with-serde both
