!include .ws/Justfile

set dotenv-load := true
set positional-arguments

server := env_var('DEPLOY_USER') + "@" + env_var('DEPLOY_HOST')
path := env_var_or_default("DEPLOY_PATH", "/srv/rustify")

generate:
  sea-orm-cli generate entity -o src/entity.example --expanded-format

validate:
  cargo clippy
  cargo test
  ansible-lint .infra/ansible/playbook.yml

fix:
  cargo fmt
  cargo clippy --fix --allow-dirty --allow-staged
  ansible-lint --write .infra/ansible/playbook.yml

build:
  docker pull rustlang/rust:nightly
  docker run --rm -it -u "$(id -u):$(id -g)" -e CARGO_INCREMENTAL=1 \
    -v "$PWD:/build/" -v "$PWD/var/docker/cargo-registry:/usr/local/cargo/registry" -w /build/ \
    rustlang/rust:nightly \
    cargo build --target=x86_64-unknown-linux-gnu --release

deploy:
  {{ this }} build
  ansible-galaxy install -r .infra/ansible/requirements.yml
  ansible-playbook -i {{ env_var('DEPLOY_HOST') }}, -u {{ env_var('DEPLOY_USER') }} -e {{ quote("deploy_path=" + path) }} .infra/ansible/playbook.yml

_deploy-old:
  cargo build -r
  ssh "{{ server }}" -- mkdir -p "{{ path }}/"
  rsync -P -e ssh "docker-compose.yml" "Dockerfile" "target/release/rustify" ".env.deploy" "proxychains.conf" "{{ server }}:{{ path }}/"
  ssh "{{ server }}" -- mkdir -p "{{ path }}/target/release"
  ssh "{{ server }}" -- cp "{{ path }}/rustify" "{{ path }}/target/release/"
  {{ this }} compose build
  {{ this }} compose up --force-recreate -d

get-db:
  scp "{{ server }}:{{ path }}/var/data.db" "var/data.db"
  scp "{{ server }}:{{ path }}/var/data.db-shm" "var/data.db-shm"
  scp "{{ server }}:{{ path }}/var/data.db-wal" "var/data.db-wal"

upload-db:
  ssh "{{ server }}" -- mkdir -p "{{ path }}/var"
  rsync -P -e ssh "var/data.db" "{{ server }}:{{ path }}/var/data.db"
  rsync -P -e ssh "var/data.db-shm" "{{ server }}:{{ path }}/var/data.db-shm"
  rsync -P -e ssh "var/data.db-wal" "{{ server }}:{{ path }}/var/data.db-wal"

compose +args:
  ssh "{{ server }}" -- docker-compose -f "{{ path }}/docker-compose.yml" {{ args }}

logs:
  {{ this }} compose logs -f

ssh:
  ssh -t "{{ server }}" "cd {{ path }}; bash --login"

watch cmd="run":
  cargo watch -c -x {{ cmd }}

xwatch cmd="run":
   x-terminal-emulator -e {{ this }} watch {{ cmd }}
