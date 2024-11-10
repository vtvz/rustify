set dotenv-load := true
set positional-arguments

just := quote(just_executable())
this := just + " -f " + quote(justfile())

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

restart:
  ssh "{{ server }}" -- docker-compose --project-directory "{{ path }}" restart bot track_check

run:
  parallel --tagstring "[{}]" --line-buffer -j2 --halt now,fail=1 cargo run --bin ::: "bot" "track_check"

watch:
  cargo watch -s 'proxychains4 -q just run'
