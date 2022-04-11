set dotenv-load := true

generate:
  sea-orm-cli generate entity -o src/entity.example --expanded-format --with-serde both

server := env_var("DEPLOY_SERVER")
path := env_var_or_default("DEPLOY_PATH", "/srv/rustify")

deploy:
  cargo build -r
  ssh "{{ server }}" -- mkdir -p "{{ path }}/"
  rsync -P -e ssh "docker-compose.yml" "Dockerfile" "target/release/rustify" ".env.deploy" "proxychains.conf" "{{ server }}:{{ path }}/"
  ssh "{{ server }}" -- mkdir -p "{{ path }}/target/release"
  ssh "{{ server }}" -- cp "{{ path }}/rustify" "{{ path }}/target/release/"
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" down
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" build
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" up -d

get-db:
  scp "{{ server }}:{{ path }}/var/data.db" "var/data.db"
  scp "{{ server }}:{{ path }}/var/data.db-shm" "var/data.db-shm"
  scp "{{ server }}:{{ path }}/var/data.db-wal" "var/data.db-wal"

upload-db:
  ssh "{{ server }}" -- mkdir -p "{{ path }}/var"
  rsync -P -e ssh "var/data.db" "{{ server }}:{{ path }}/var/data.db"

logs:
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" logs -f

ssh:
  ssh -t "{{ server }}" "cd {{ path }}; bash --login"

watch cmd="run":
  cargo watch -c -x {{ cmd }}

xwatch cmd="run":
   x-terminal-emulator -e just watch {{ cmd }}
