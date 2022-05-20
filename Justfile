set dotenv-load := true

server := env_var('DEPLOY_USER') + "@" + env_var('DEPLOY_HOST')
path := env_var_or_default("DEPLOY_PATH", "/srv/rustify")

just := quote(just_executable())
this := just + " -f " + quote(justfile())

generate:
  sea-orm-cli generate entity -o src/entity.example --expanded-format --with-serde both

validate:
  cargo clippy
  cargo test
  ansible-lint .infra/ansible/playbook.yml

fix:
  cargo fmt
  cargo clippy --fix --allow-dirty --allow-staged
  ansible-lint --write .infra/ansible/playbook.yml

deploy:
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
