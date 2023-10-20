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

docker-push:
	docker push "ghcr.io/vtvz/rustify:{{ `git describe --abbrev=10 --always` }}"

docker-build:
	docker build --progress plain -t "ghcr.io/vtvz/rustify:{{ `git describe --abbrev=10 --always` }}" .

deploy:
  cd _infra/ansible && just deploy "ghcr.io/vtvz/rustify:{{ `git describe --abbrev=10 --always` }}"

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
