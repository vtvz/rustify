set dotenv-load := true

generate:
  sea-orm-cli generate entity -o src/entity --expanded-format --with-serde both

server := env_var("DEPLOY_SERVER")
path := env_var_or_default("DEPLOY_PATH", "/srv/rustify")

deploy:
  cargo build -r
  ssh "{{ server }}" -- mkdir -p "{{ path }}/"
  rsync -P -e ssh "docker-compose.yml" "Dockerfile" "target/release/rustify" ".env.deploy" "{{ server }}:{{ path }}/"
  ssh "{{ server }}" -- mkdir -p "{{ path }}/target/release"
  ssh "{{ server }}" -- cp "{{ path }}/rustify" "{{ path }}/target/release/"
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" down
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" build
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" up -d

logs:
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" logs -f
